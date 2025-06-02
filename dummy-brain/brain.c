#include "brain.h"
#include <stdlib.h>
#include <time.h>
#include <string.h>
#include <math.h>
#include <stdio.h>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

// Constants from Rust brain (approximated or simplified)
#define TO_COLONY 0
#define TO_FOOD 1
#define BASIC_PHEROMONE_LAYED_AMOUNTY 5.0f // Simplified constant
#define MAX_TURN_ANGLE (M_PI / 4.0f)

void setup(PlayerSetup* setup) {
    srand((unsigned)time(NULL));
    setup->decay_rates[TO_COLONY] = 0.99f;
    setup->decay_rates[TO_FOOD] = 0.9f;
    printf("Hello from dummy brain `setup` func\n");
}

void update(const AntInput* input, uint8_t memory[MEMORY_SIZE], AntOutput* output) {
    // --- Pheromone Laying Decision ---
    int pheromone_channel_to_lay = input->is_carrying_food ? TO_FOOD : TO_COLONY;

    // --- Pheromone Amount Calculation ---
    float layed_pheromone_amount = BASIC_PHEROMONE_LAYED_AMOUNTY;

    // --- Initialize Pheromone Output ---
    output->pheromone_amounts[pheromone_channel_to_lay] = layed_pheromone_amount;

    // --- Movement Decision ---
    float turn_angle;

    if (input->is_carrying_food) {
        if (input->colony_sense[1] >= 0.0f) { // Check if distance is valid (not -1.0)
            turn_angle = input->colony_sense[0]; // Turn towards colony
        } else if (input->pheromone_senses[TO_COLONY][1] > 0.0f) {
            turn_angle = input->pheromone_senses[TO_COLONY][0]; // Follow "to colony" trail
        } else {
            // Random turn
            turn_angle = ((float)rand() / (float)RAND_MAX) * 2.0f * MAX_TURN_ANGLE - MAX_TURN_ANGLE;
        }
    } else { // Not carrying food
        if (input->food_sense[1] >= 0.0f) { // Check if distance is valid (not -1.0)
            turn_angle = input->food_sense[0]; // Turn towards food
        } else if (input->pheromone_senses[TO_FOOD][1] > 0.0f) {
            turn_angle = input->pheromone_senses[TO_FOOD][0]; // Follow "to food" trail
        } else {
            // Random turn
            turn_angle = ((float)rand() / (float)RAND_MAX) * 2.0f * MAX_TURN_ANGLE - MAX_TURN_ANGLE;
        }
    }

    output->turn_angle = turn_angle;

    // If an enemy is detected within 5 cells
    if (input->enemy_sense[1] >= 0.0f && input->enemy_sense[1] < 5.0f) {
        printf("Enemy detected at angle: %f, distance: %f\n", input->enemy_sense[0], input->enemy_sense[1]);
        output->turn_angle = input->enemy_sense[0]; // Turn towards enemy
    } 
    
    output->try_attack = true;
}
