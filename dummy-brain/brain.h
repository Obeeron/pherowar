// File: brain.h
// Defines the C Application Binary Interface (ABI) for PheroWar player AIs.

#pragma once

#include <stdint.h> // For uint8_t
#include <stdbool.h>  // For bool type

// Defines the size in bytes of the persistent memory block available to each individual ant.
// Each ant has its own dedicated memory array of this size. This memory persists across
// multiple calls to the update function for that specific ant, allowing it to store state,
// remember information, or implement more complex behaviors over its lifespan.
// The memory is initialized to zeros when an ant is spawned.
#define MEMORY_SIZE 32

// Provides all sensory information and state data for an ant from the game simulation.
// This structure is passed as read-only input to the update function for each ant.
// It contains information about the ant's current status (e.g., carrying food, on colony),
// its perception of the environment (e.g., pheromones, walls, food, enemies), its remaining
// longevity, and whether it's currently engaged in combat.
// Many senses operate within a forward-facing arc defined by SENSE_MAX_ANGLE (typically PI/4 radians
// or 45 degrees to each side of the ant's orientation) and up to SENSE_MAX_DISTANCE (typically 10.0 units).
typedef struct {
    // True if the ant is currently carrying a piece of food, false otherwise.
    bool is_carrying_food;

    // True if the ant is currently located on its own colony's nest cell, false otherwise.
    bool is_on_colony;

    // True if the ant is currently located on a cell containing a food source, false otherwise.
    bool is_on_food;

    // pheromone_senses[8][2]:
    // Sensory data for each of the 8 available pheromone channels, detected by sampling points within the ant's forward arc.
    // - pheromone_senses[channel_index][0]: Relative angle (in radians) from the ant's current orientation
    //   to the strongest perceived signal for channel_index. Positive values are counter-clockwise.
    // - pheromone_senses[channel_index][1]: Strength of that detected pheromone signal for channel_index.
    //   Strengths range from 0.0 (no signal) up to MAX_PHEROMONE_AMOUNT (typically 255.0).
    //   If no signal for a channel is detected in the arc, both angle and strength may be 0.0.
    float pheromone_senses[8][2];

    // cell_sense[8]:
    // Strength of each of the 8 pheromone channels directly in the grid cell currently occupied by the ant.
    // - cell_sense[channel_index]: The amount of pheromone for channel_index present in the ant's current cell.
    //   Values range from 0.0 up to MAX_PHEROMONE_AMOUNT (typically 255.0).
    float cell_sense[8];

    // wall_sense[2]:
    // Sensory data for the most prominent wall segment detected in the ant's forward-facing arc.
    // - wall_sense[0]: Relative angle (in radians) from the ant's current orientation to the wall segment.
    // - wall_sense[1]: Distance in tiles to the wall segment. Value is -1.0 if no wall is detected.
    float wall_sense[2];

    // food_sense[2]:
    // Sensory data for the most prominent food source detected in the ant's forward-facing arc.
    // - food_sense[0]: Relative angle (in radians) from the ant's current orientation to the food source.
    // - food_sense[1]: Distance in tiles to the food source. Value is -1.0 if no food is detected.
    float food_sense[2];

    // colony_sense[2]:
    // Sensory data for the ant's own colony nest, sensed up to SENSE_MAX_DISTANCE (if not occluded by a wall).
    // - colony_sense[0]: Relative angle (in radians) from the ant's current orientation to its nest.
    // - colony_sense[1]: Distance in tiles to the nest. Value is -1.0 if the nest is beyond SENSE_MAX_DISTANCE or occluded.
    float colony_sense[2];

    // enemy_sense[2]:
    // Sensory data for the most prominent enemy ant detected in the current cell or forward-facing arc.
    // - enemy_sense[0]: Relative angle (in radians) from the ant's current orientation to the enemy.
    //   This will be 0.0 if the enemy is in the same cell.
    // - enemy_sense[1]: Distance in tiles to the enemy. Value is -1.0 if no enemy is detected.
    //   If the enemy is in the same cell, distance might be 0.0 or a very small positive value.
    float enemy_sense[2];

    // longevity: Remaining lifespan of the ant, also serves as its health in combat.
    // Value ranges from MAX_ANT_LONGEVITY (e.g., 300.0) down to 0.0 (death).
    // Ants can rejuvenate longevity by delivering food to the nest or by winning fights.
    float longevity;

    // is_fighting: True if the ant is currently engaged in combat (e.g., has an active opponent list in the simulation), false otherwise.
    bool is_fighting;
} AntInput;

// AntOutput:
// Defines the actions an ant intends to perform in the current simulation tick.
// This structure is populated by the player's update function to command the ant.
// It includes the desired turning angle, amounts of pheromones to lay, and intent to attack.
typedef struct {
    // turn_angle: The relative angle (in radians) the ant should turn.
    // Positive values indicate a counter-clockwise turn, negative values indicate a clockwise turn.
    // The simulation will attempt to apply this turn. 
    float turn_angle;

    // pheromone_amounts[8]: An array specifying the amount of pheromone to deposit for each of the 8 channels in the current cell.
    // pheromone_amounts[channel_index] is the amount for that channel.
    // Values should be between 0.0 (no pheromone) and MAX_PHEROMONE_AMOUNT (e.g., 255.0).
    // The actual amount deposited might be capped or affected by simulation factors.
    float pheromone_amounts[8];

    // try_attack: Boolean indicating the ant's intent to attack.
    // If true, the simulation will attempt to initiate or continue combat with an enemy ant
    // if one is present in the same cell or a suitable target is otherwise determined by the simulation rules.
    bool try_attack;
} AntOutput;

// PlayerSetup:
// Used by the player AI to configure colony-specific parameters at the start of the game.
// This structure is passed to the setup function, allowing the AI to customize aspects
// like pheromone decay rates for its colony.
typedef struct {
    // decay_rates[8]: Array to set the decay rates for each of the 8 pheromone channels for this player's colony.
    // decay_rates[channel_index] defines the rate for that channel.
    // A value represents the fraction of pheromone strength that REMAINS after 1 second of simulation time.
    // For example:
    //  - 1.0 means no decay (pheromone is permanent).
    //  - 0.95 means 95% of the pheromone strength remains after 1 second (5% decays).
    //  - 0.0 means the pheromone decays completely within 1 second (or the decay interval).
    // These rates are applied by the simulation at regular intervals (e.g., PHEROMONE_DECAY_INTERVAL).
    float decay_rates[8];
} PlayerSetup;

// setup:
// Initializes the player's ant colony AI.
// This function is called once by the game engine when the player's AI is first loaded,
// before the simulation begins for this colony. It provides an opportunity for the AI to
// perform initial setup, such as configuring pheromone decay rates via the setup_info parameter.
// Parameters:
//   setup_info: A pointer to a PlayerSetup struct. The AI should modify the fields of this
//               struct (e.g., decay_rates) to configure its colony's parameters.
//               The game engine will use these modified values.
void setup(PlayerSetup* setup_info);

// update:
// The core decision-making function for an individual ant.
// This function is called repeatedly by the game engine for each ant belonging to the player's
// colony, typically every "think" tick of the simulation (e.g., related to THINK_INTERVAL).
// Based on the provided input (sensory data and ant state) and its persistent memory,
// the AI must decide what action the ant should take and write these actions to the output struct.
// Parameters:
//   input: A pointer to an AntInput struct containing read-only information about the ant's
//          current state and its perception of the environment.
//   memory: A pointer to a block of MEMORY_SIZE bytes (currently 32 bytes) of uint8_t data.
//           This memory is persistent for this specific ant across multiple calls to update.
//           It is initialized to zeros when the ant spawns. The AI can read from and write to this memory
//           to store state or other information.
//   output: A pointer to an AntOutput struct where the AI must write the ant's desired actions
//           for the current simulation tick (e.g., turning angle, pheromones to lay, attack intent).
void update(const AntInput* input, uint8_t memory[MEMORY_SIZE], AntOutput* output);