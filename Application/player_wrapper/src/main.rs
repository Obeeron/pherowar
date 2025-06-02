use libloading::{Library, Symbol};
use rkyv::{from_bytes, rancor::Error, to_bytes};
use shared::{AntInput, AntOutput, AntRequest, AntResponse, PlayerSetup};
use std::io::{Read, Write};
use std::os::unix::net::UnixListener;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[player] Loading brain.so...");
    let lib = match unsafe { Library::new("./brain.so") } {
        Ok(lib) => lib,
        Err(e) => {
            eprintln!("[player][error] Failed to load brain.so: {}", e);
            return Err(Box::new(e));
        }
    };
    println!("[player] brain.so loaded successfully.");

    let update_func: Symbol<unsafe extern "C" fn(*const AntInput, *mut u8, *mut AntOutput)> =
        match unsafe { lib.get(b"update") } {
            Ok(sym) => {
                println!("[player] 'update' symbol loaded.");
                sym
            }
            Err(e) => {
                eprintln!("[player][error] Failed to load 'update' symbol: {}", e);
                return Err(Box::new(e));
            }
        };
    let setup_func: Symbol<unsafe extern "C" fn(*mut PlayerSetup)> =
        match unsafe { lib.get(b"setup") } {
            Ok(sym) => {
                println!("[player] 'setup' symbol loaded.");
                sym
            }
            Err(e) => {
                eprintln!("[player][error] Failed to load 'setup' symbol: {}", e);
                return Err(Box::new(e));
            }
        };

    let listener = match UnixListener::bind("/tmp/pherowar/pherowar.sock") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[player][error] Failed to bind unix socket: {}", e);
            return Err(Box::new(e));
        }
    };
    println!("[player] Waiting for host to connect...");
    let (mut stream, _) = match listener.accept() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[player][error] Failed to accept connection: {}", e);
            return Err(Box::new(e));
        }
    };
    println!("[player] Connected to pherowar host.");

    /* --------------------------------------------------
     *  Send PlayerSetup to the host
     * -------------------------------------------------- */
    let mut setup = PlayerSetup {
        decay_rates: [0.9; 8],
    };
    unsafe { setup_func(&mut setup) };

    let bytes = to_bytes::<Error>(&setup)?; // rkyv encode
    stream.write_all(&(bytes.len() as u32).to_le_bytes())?;
    stream.write_all(&bytes)?;
    println!("[player] Setup sent to host.");

    /* wait for “hello player” from the host (unchanged) */
    let mut buf = [0u8; 64];
    let n = stream.read(&mut buf)?;
    println!(
        "[player] Received from host: {}",
        String::from_utf8_lossy(&buf[..n])
    );

    /* --------------------------------------------------
     *  Main request/response loop (rkyv ⇄ rkyv)
     * -------------------------------------------------- */
    loop {
        /* ---- receive request ---- */
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).is_err() {
            break;
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        if len > 256 {
            eprintln!("[player] oversized AntRequest");
            break;
        }
        let mut req_buf = vec![0u8; len];
        stream.read_exact(&mut req_buf)?;

        let ant_req: AntRequest = match from_bytes::<AntRequest, Error>(&req_buf) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[player] invalid AntRequest: {e}");
                break;
            }
        };

        /* ---- run user brain ---- */
        let mut memory = ant_req.memory;
        let mut output = AntOutput {
            turn_angle: 0.0,
            pheromone_amounts: [0.0; 8],
            try_attack: false,
        };
        unsafe { update_func(&ant_req.input, memory.as_mut_ptr(), &mut output) };
        let ant_resp = AntResponse { output, memory };

        /* ---- encode & send response ---- */
        let resp_bytes = to_bytes::<Error>(&ant_resp)?;
        stream.write_all(&(resp_bytes.len() as u32).to_le_bytes())?;
        stream.write_all(&resp_bytes)?;
    }

    println!("[player] Exiting main loop.");
    Ok(())
}
