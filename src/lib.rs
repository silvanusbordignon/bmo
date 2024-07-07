use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::thread::{self, JoinHandle};
use std::sync::mpsc;
use std::time::Duration;

use rand::{Rng, thread_rng};

use robotics_lib::energy::Energy;
use robotics_lib::event::events::Event;
use robotics_lib::runner::{Robot, Runnable};
use robotics_lib::runner::backpack::BackPack;
use robotics_lib::world::coordinates::Coordinate;
use robotics_lib::world::World;
use robotics_lib::interface::{destroy, Direction, go, robot_view, put};
use robotics_lib::utils::{go_allowed, LibError};

use olympus::channel::Channel;

use cargo_commandos_lucky::lucky_function::lucky_spin;
use crab_rave_explorer::direction::RichDirection;
use macroquad::rand::ChooseRandom;
use macroquad::window::next_frame;
use op_map::op_pathfinding::{OpActionInput, OpActionOutput};

use oxagaudiotool::OxAgAudioTool;
use oxagaudiotool::sound_config::OxAgSoundConfig;
use robotics_lib::world::tile::Content;

// MentalState defines a set of emotions for the robot
enum MentalState {
    Happy,
    Calm,
    Sad,
    Panic,
}

// State change probabilities
const CHANCE_CALM_TO_HAPPY  :f64 = 0.01;
const CHANCE_HAPPY_TO_CALM  :f64 = 0.1;
const CHANCE_CALM_TO_SAD    :f64 = 0.01;
const CHANCE_SAD_TO_CALM    :f64 = 0.015;
const CHANCE_SAD_TO_PANIC   :f64 = 0.005;
const CHANCE_PANIC_TO_SAD   :f64 = 0.09;

// Structs used in the main -> worker thread communications

#[derive(PartialEq)]
enum Command {
    PLAY(Sound),
    STOP
}

#[derive(PartialEq)]
enum Sound {
    HAPPY,
    CALM,
    SAD,
    PANIC
}

// Below is the robot's implementation

pub struct BMO {
    robot: Robot,
    channel: Rc<RefCell<Channel>>,
    mental_state: MentalState,
    tx_channel: mpsc::Sender<Command>,
    calm_moves: Vec<RichDirection>
}

impl BMO {
    pub fn new(channel: Rc<RefCell<Channel>>) -> BMO {

        let (tx, rx) = mpsc::channel();

        // Worker thread that will handle the audio

        let handle = thread::spawn(move || {

            let mut audio = OxAgAudioTool::new(
                HashMap::new(),
                HashMap::new(),
                HashMap::new()
            ).unwrap();

            let background_music = OxAgSoundConfig::new_looped_with_volume("assets/audio/background.mp3", 0.25);
            audio.play_audio(&background_music).unwrap();

            loop {
                match rx.recv() {
                    Ok(command) => {
                        match command {
                            Command::PLAY(sound) => match sound {
                                Sound::HAPPY => { let _ = audio.play_audio(&OxAgSoundConfig::new("assets/audio/happy.mp3")); },
                                Sound::CALM => { let _ = audio.play_audio(&OxAgSoundConfig::new("assets/audio/calm.mp3")); },
                                Sound::SAD => { let _ = audio.play_audio(&OxAgSoundConfig::new("assets/audio/sad.mp3")); },
                                Sound::PANIC => { let _ = audio.play_audio(&OxAgSoundConfig::new("assets/audio/panic.mp3")); }
                            },
                            Command::STOP => ()
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });

        BMO {
            robot: Robot::default(),
            channel,
            mental_state: MentalState::Calm,
            tx_channel: tx,
            calm_moves: Vec::new()
        }
    }
}

impl Runnable for BMO {
    fn process_tick(&mut self, world: &mut World) {

        // Robot view in order to allow tools to work properly
        let _ = robot_view(self, world);

        // Choose how to act based on the robot's mental state
        match self.mental_state {
            MentalState::Happy => happy_routine(self, world),
            MentalState::Calm => calm_routine(self, world),
            MentalState::Sad => sad_routine(self, world),
            MentalState::Panic => panic_routine(self, world)
        }

        // You need to call this method to update the GUI
        self.channel.borrow_mut().send_game_info(self, world);
    }


    fn handle_event(&mut self, event: Event) {

        match event {
            Event::Ready => {}
            Event::Terminated => {}
            Event::TimeChanged(weather) => {
                // Update the GUI
                self.channel.borrow_mut().send_weather_info(weather);
            }
            Event::DayChanged(_) => {}
            Event::EnergyRecharged(_) => {}
            Event::EnergyConsumed(_) => {}
            Event::Moved(_, (_, _)) => {}
            Event::TileContentUpdated(_, _) => {}
            Event::AddedToBackpack(_, _) => {}
            Event::RemovedFromBackpack(_, _) => {}
        }
    }

    fn get_energy(&self) -> &Energy { &self.robot.energy }
    fn get_energy_mut(&mut self) -> &mut Energy { &mut self.robot.energy }
    fn get_coordinate(&self) -> &Coordinate { &self.robot.coordinate }
    fn get_coordinate_mut(&mut self) -> &mut Coordinate { &mut self.robot.coordinate }
    fn get_backpack(&self) -> &BackPack { &self.robot.backpack }
    fn get_backpack_mut(&mut self) -> &mut BackPack { &mut self.robot.backpack }
}

fn happy_routine(robot: &mut BMO, world: &mut World) {

    match lucky_spin(&mut robot.robot) {
        Ok(message) => println!("{message}"),
        Err(LibError::NotEnoughEnergy) => eprintln!("Not enough energy to lucky spin!"),
        Err(_) => eprintln!("ERROR: call to lucky spin")
    }

    // Transition back to calm
    if thread_rng().gen_bool(CHANCE_HAPPY_TO_CALM) {

        eprintln!("---- CALM ----");

        robot.tx_channel.send(Command::PLAY(Sound::CALM)).unwrap();
        robot.mental_state = MentalState::Calm;
    }
}

fn calm_routine(robot: &mut BMO, world: &mut World) {

    // When the robot's calm, there's not much going on in its mind. There's a subtle interest in
    // novelty, which the robots expresses by exploring new tiles; this is done by using a tool

    if robot.calm_moves.len() == 0 {
        match crab_rave_explorer::algorithm::cheapest_border(world, robot) {
            None => eprintln!("Calm - Robot cannot move using explorer"),
            Some(v) => match crab_rave_explorer::algorithm::move_to_cheapest_border(world, robot, v) {
                Ok(_) => (),
                Err((leftover_moves, err)) => {
                    eprintln!("Calm - Robot did not move because of {:?}", err);
                    robot.calm_moves = leftover_moves
                }
            }
        }
    }
    else {
        match crab_rave_explorer::algorithm::move_to_cheapest_border(world, robot, robot.calm_moves.clone()) {
            Ok(_) => (),
            Err((leftover_moves, err)) => {
                eprintln!("Calm - Robot did not move because of {:?}", err);
                robot.calm_moves = leftover_moves
            }
        }
        robot.calm_moves = Vec::new();
    }

    // Transitions to either sad or happy
    // Priority is given to the former

    if thread_rng().gen_bool(CHANCE_CALM_TO_SAD) {

        eprintln!("---- SAD ----");

        robot.mental_state = MentalState::Sad;
        robot.tx_channel.send(Command::PLAY(Sound::SAD)).unwrap();
    }
    if thread_rng().gen_bool(CHANCE_CALM_TO_HAPPY) {

        eprintln!("---- HAPPY ----");

        robot.mental_state = MentalState::Happy;
        robot.tx_channel.send(Command::PLAY(Sound::HAPPY)).unwrap();
    }
}

fn sad_routine(robot: &mut BMO, world: &mut World) {

    // When sad, BMO doesn't really think about what to do. It basically goes in autopilot mode and
    // tries to do something useful, even though it will not resolve any of its internal conflicts

    // In this context, this "autopilot" mode is a tool called op_map, and the "something useful"
    // corresponds to actions given to the tool which can be considered beneficial for the robot

    let mut shopping_list = op_map::op_pathfinding::ShoppingList::new(Vec::new());

    // If the backpack's already full it makes no sense trying to destroy a tree

    if robot.get_backpack().get_size() != 20 {
        shopping_list.list.push((Content::Tree(1), Some(OpActionInput::Destroy())));
    }
    shopping_list.list.push((Content::Crate(Range::default()), Some(OpActionInput::Put(Content::Tree(1), 1))));


    // For each item in the shopping list
    while shopping_list.list.len() > 0 {

        // Time delay helping the visualizer on my machine
        thread::sleep(Duration::from_millis(100));

        // Find the best action for that item in the shopping list
        match op_map::op_pathfinding::get_best_action_to_element(robot, world, &mut shopping_list) {
            Some(next_action) => {

                eprintln!("Sad | op_map next action {:?}", next_action);

                match next_action {
                    OpActionOutput::Move(dir) => {
                        go(robot, world, dir).expect("Sad - op_map can't move");
                    }
                    OpActionOutput::Destroy(dir) => {
                        match destroy(robot, world, dir) {
                            Ok(_) => (),
                            Err(LibError::NotEnoughSpace(x)) => eprintln!("Sad - no space to destroy"),
                            Err(e) => eprintln!("Sad - destroy {:?}", e)
                        }
                    }
                    OpActionOutput::Put(c, u, d) => {
                        match put(robot, world, c, u, d) {
                            Ok(_) => (),
                            Err(LibError::OperationNotAllowed) => eprintln!("Sad - put not allowed"),
                            Err(e) => eprintln!("Sad - put {:?}", e)
                        }
                    }
                }
            },
            None => { eprintln!("Sad | op_map no action, no content probably"); shopping_list.print_shopping_list(); break }
        }
    }

    // Transitions either back to calm or to panic
    // Priority is given to the former

    if thread_rng().gen_bool(CHANCE_SAD_TO_CALM) {

        eprintln!("---- CALM ----");

        robot.mental_state = MentalState::Calm;
        robot.tx_channel.send(Command::PLAY(Sound::CALM)).unwrap();
    }
    if thread_rng().gen_bool(CHANCE_SAD_TO_PANIC) {

        eprintln!("---- PANIC ----");

        robot.mental_state = MentalState::Panic;
        robot.tx_channel.send(Command::PLAY(Sound::PANIC)).unwrap();
    }
}

fn panic_routine(robot: &mut BMO, world: &mut World) {

    // When panicking, BMO doesn't know what to do, and just moves to a random nearby tile

    let directions = vec![
        Direction::Up,
        Direction::Left,
        Direction::Down,
        Direction::Right
    ];
    let dir = directions.choose().unwrap();

    match go_allowed(robot, world, dir) {
        Ok(_) => {
            let _ = go(robot, world, dir.clone());
        },
        Err(_) => ()
    }

    // Transition back to sad
    if thread_rng().gen_bool(CHANCE_PANIC_TO_SAD) {

        eprintln!("---- SAD ----");

        robot.mental_state = MentalState::Sad;
        robot.tx_channel.send(Command::PLAY(Sound::SAD)).unwrap();
    }
}
