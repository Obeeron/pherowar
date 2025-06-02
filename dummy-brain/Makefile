CC = gcc
CFLAGS = -O3 -std=c11 -fPIC
TARGET = brain.so

all: $(TARGET)

brain.so: brain.c brain.h
	$(CC) $(CFLAGS) -shared -o brain.so brain.c

clean:
	rm -f $(TARGET)
