//use physics::newtonian::state_vars::Position;

pub trait Newtonian {
    fn x(&self) -> f64;
    fn vx(&self) -> f64;
    //    fn add_force(&self, fx: f64);
    fn step(&mut self);
}

//mod state_vars {
pub struct Position {
    x: f64,
}

pub struct Velocity {
    x: f64,
}

impl Position {
    pub fn new(x: f64) -> Position {
        Position { x }
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    fn plus(&self, v: &Velocity) -> Position {
        Position::new(self.x + v.x)
    }
}

impl Velocity {
    pub fn new(x: f64) -> Velocity {
        Velocity { x }
    }

    pub fn x(&self) -> f64 {
        self.x
    }
}
//}

pub struct NewtonianState {
    pub position: Position,
    pub velocity: Velocity,
//    pub mass: f64,
}

impl NewtonianState {
    fn new(x: f64, vx: f64) -> NewtonianState {
        NewtonianState { position: Position::new(x), velocity: Velocity::new(vx) }
    }
}

impl Newtonian for NewtonianState {
    fn x(&self) -> f64 {
        self.position.x()
    }

    fn vx(&self) -> f64 {
        self.velocity.x()
    }

    fn step(&mut self) {
        self.position = self.position.plus(&self.velocity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stationary() {
        let mut subject = SimpleNewtonian::new(0.0, 0.0);
        subject.step();
        assert_eq!(0.0, subject.x());
        assert_eq!(0.0, subject.vx());
    }

    #[test]
    fn coasting() {
        let mut subject = SimpleNewtonian::new(0.0, 1.0);
        subject.step();
        assert_eq!(1.0, subject.x());
        assert_eq!(1.0, subject.vx());
    }

    struct SimpleNewtonian {
        state: NewtonianState,
    }

    impl SimpleNewtonian {
        fn new(x: f64, vx: f64) -> SimpleNewtonian {
            SimpleNewtonian {
                state: NewtonianState::new(x, vx)
            }
        }
    }

    impl Newtonian for SimpleNewtonian {
        fn x(&self) -> f64 {
            self.state.x()
        }

        fn vx(&self) -> f64 {
            self.state.vx()
        }

        fn step(&mut self) {
            self.state.step();
        }
    }
}
