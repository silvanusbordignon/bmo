use std::cell::RefCell;
use std::rc::Rc;
use rip_worldgenerator::MyWorldGen;
use robotics_lib::energy::Energy;
use robotics_lib::event::events::Event;
use robotics_lib::runner::{Robot, Runnable};
use robotics_lib::runner::backpack::BackPack;
use robotics_lib::world::coordinates::Coordinate;
use robotics_lib::world::World;
use olympus::channel::Channel;
use olympus::Visualizer;
use macroquad::{prelude::*};
use robotics_lib::interface::{Direction, go};
use robotics_lib::utils::go_allowed;
use macroquad::rand::ChooseRandom;

struct BMO {
    robot: Robot,
    channel: Rc<RefCell<Channel>>
}

impl BMO {
    fn new(channel: Rc<RefCell<Channel>>) -> BMO {
        BMO {
            robot: Robot::default(),
            channel
        }
    }
}

impl Runnable for BMO {
    fn process_tick(&mut self, world: &mut World) {
        let directions = vec![
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right
        ];
        let dir = directions.choose().unwrap();

        match go_allowed(self, world, dir) {
            Ok(_) => {
                let _ = go(self, world, dir.clone());
            },
            Err(_) => ()
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

fn window_conf() -> Conf {
    Conf {
        window_title: "Olympus".to_string(),
        window_width: 1920,
        window_height: 1080,
        fullscreen: false,
        ..Default::default()
    }
}

// Macroquad entry point
#[macroquad::main(window_conf)]
async fn main() {
    // Channel used by the robot to comunicate with the GUI
    let channel = Rc::new(RefCell::new(Channel::default()));
    let robot = BMO::new(Rc::clone(&channel));

    let world_size = 200;
    let world_generator = MyWorldGen::new_param(
        world_size,
        5,
        3,
        3,
        true,
        true,
        3,
        false,
        None
    );

    // Visualizer
    let mut visualizer = Visualizer::new(robot, world_generator, world_size, Rc::clone(&channel));
    visualizer.start().await;
}
