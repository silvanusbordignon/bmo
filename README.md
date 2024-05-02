# BMO

This repository is supposed to contain the AI for the Advanced Programming course (2023/2024, UniTN, prof. Patrignani). This project is to be completed in the future and turned in on a future exam session.

## Project idea

BMO is a small, kind-hearted robot. One day it wakes up in a new world, and quickly realized it's the only self-conscious being left alive in the land. Nowadays it lives alone in the world, wondering around when it's calm, or trying to resist it's strongest emotions, sadness from being alone, and a sort of panic that ensues every once in a while.

## Technical component

The robot is implemented with the **typestate pattern**. Each emotion that the robot feels is encoded as a state, and any change in its mental state is encoded as a transition between two of these states. The states in which the robot can be are:
- **calm**: when BMO is calm, he wonders around the map and interacts with it
- **sadness**: when sadness ensues, the robot moves less and less, often spending much of its time thinking about his life
- **panic**: when BMO feels the most helpless, it gets into a sort of desparate state, running around frantically and destroying most of the items it finds in its path 