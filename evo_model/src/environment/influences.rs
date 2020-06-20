use crate::biology::cell::Cell;
use crate::environment::local_environment::*;
use crate::physics::bond::*;
use crate::physics::newtonian::*;
use crate::physics::overlap::*;
use crate::physics::quantities::*;
use crate::physics::shapes::Circle;
use crate::physics::sortable_graph::*;
use crate::physics::spring::*;
use crate::physics::util::*;
use log::trace;

pub trait Influence {
    fn apply(
        &self,
        cell_graph: &mut SortableGraph<Cell, Bond, AngleGusset>,
        subtick_duration: Duration,
    );
}

#[derive(Debug)]
pub struct WallCollisions {
    walls: Walls,
    spring: Box<dyn Spring>,
}

impl WallCollisions {
    pub fn new(min_corner: Position, max_corner: Position, spring: Box<dyn Spring>) -> Self {
        WallCollisions {
            walls: Walls::new(min_corner, max_corner),
            spring,
        }
    }

    pub fn collision_force(mass: Mass, velocity: Velocity, overlap: Displacement) -> Force {
        Force::new(
            Self::x_or_y_collision_force(mass, velocity.x(), overlap.x()),
            Self::x_or_y_collision_force(mass, velocity.y(), overlap.y()),
        )
    }

    fn x_or_y_collision_force(mass: Mass, velocity: f64, overlap: f64) -> f64 {
        let v = if overlap > 0.0 {
            velocity.max(overlap)
        } else if overlap < 0.0 {
            velocity.min(overlap)
        } else {
            -velocity
        };
        -mass.value() * (velocity + v)
    }
}

impl Influence for WallCollisions {
    fn apply(
        &self,
        cell_graph: &mut SortableGraph<Cell, Bond, AngleGusset>,
        subtick_duration: Duration,
    ) {
        let overlaps = self.walls.find_overlaps(cell_graph);
        for (handle, overlap) in overlaps {
            let cell = cell_graph.node_mut(handle);
            cell.environment_mut().add_overlap(overlap);
            let force = if subtick_duration == Duration::new(1.0) {
                Self::collision_force(cell.mass(), cell.velocity(), -overlap.incursion())
            } else {
                overlap.to_force(&*self.spring)
            };
            trace!("Cell {} Wall {:?}", cell.node_handle(), force);
            cell.forces_mut().add_force(force);
        }
    }
}

#[derive(Debug)]
pub struct PairCollisions {
    spring: Box<dyn Spring>,
}

impl PairCollisions {
    #[allow(clippy::new_without_default)]
    pub fn new(spring: Box<dyn Spring>) -> Self {
        PairCollisions { spring }
    }
}

impl Influence for PairCollisions {
    fn apply(
        &self,
        cell_graph: &mut SortableGraph<Cell, Bond, AngleGusset>,
        _subtick_duration: Duration,
    ) {
        let overlaps = find_pair_overlaps(cell_graph);
        for (handle, overlap) in overlaps {
            let cell = cell_graph.node_mut(handle);
            cell.environment_mut().add_overlap(overlap);
            let force = overlap.to_force(&*self.spring);
            trace!("Cell {} Pair {:?}", cell.node_handle(), force);
            cell.forces_mut().add_force(force);
        }
    }
}

#[derive(Debug)]
pub struct BondForces {}

impl BondForces {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        BondForces {}
    }
}

impl Influence for BondForces {
    fn apply(
        &self,
        cell_graph: &mut SortableGraph<Cell, Bond, AngleGusset>,
        _subtick_duration: Duration,
    ) {
        let strains = calc_bond_strains(cell_graph);
        for (handle, strain) in strains {
            let cell = cell_graph.node_mut(handle);
            let force = strain.to_force();
            trace!("Cell {} Bond {:?}", cell.node_handle(), force);
            cell.forces_mut().add_force(force);
        }
    }
}

#[derive(Debug)]
pub struct BondAngleForces {}

impl BondAngleForces {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        BondAngleForces {}
    }
}

impl Influence for BondAngleForces {
    fn apply(
        &self,
        cell_graph: &mut SortableGraph<Cell, Bond, AngleGusset>,
        _subtick_duration: Duration,
    ) {
        let forces = calc_bond_angle_forces(cell_graph);
        for (handle, force) in forces {
            let cell = cell_graph.node_mut(handle);
            trace!("Cell {} BondAngle {:?}", cell.node_handle(), force);
            cell.forces_mut().add_force(force);
        }
    }
}

pub struct SimpleForceInfluence {
    influence_force: Box<dyn SimpleInfluenceForce>,
}

impl SimpleForceInfluence {
    pub fn new(influence_force: Box<dyn SimpleInfluenceForce>) -> Self {
        SimpleForceInfluence { influence_force }
    }
}

impl Influence for SimpleForceInfluence {
    fn apply(
        &self,
        cell_graph: &mut SortableGraph<Cell, Bond, AngleGusset>,
        subtick_duration: Duration,
    ) {
        for cell in cell_graph.nodes_mut() {
            let force = self.influence_force.calc_force(cell, subtick_duration);
            cell.forces_mut().add_force(force);
        }
    }
}

pub trait SimpleInfluenceForce {
    fn calc_force(&self, cell: &Cell, subtick_duration: Duration) -> Force;
}

#[derive(Debug)]
pub struct ConstantForce {
    force: Force,
}

impl ConstantForce {
    pub fn new(force: Force) -> Self {
        ConstantForce { force }
    }
}

impl SimpleInfluenceForce for ConstantForce {
    fn calc_force(&self, _ball: &Cell, _subtick_duration: Duration) -> Force {
        self.force
    }
}

#[derive(Debug)]
pub struct WeightForce {
    gravity: Acceleration,
}

impl WeightForce {
    pub fn new(gravity: f64) -> Self {
        WeightForce {
            gravity: Acceleration::new(0.0, gravity),
        }
    }
}

impl SimpleInfluenceForce for WeightForce {
    fn calc_force(&self, cell: &Cell, _subtick_duration: Duration) -> Force {
        let force = cell.mass() * self.gravity;
        trace!("Cell {} Weight {:?}", cell.node_handle(), force);
        force
    }
}

#[derive(Debug)]
pub struct BuoyancyForce {
    gravity: Acceleration,
    fluid_density: Density,
}

impl BuoyancyForce {
    pub fn new(gravity: f64, fluid_density: f64) -> Self {
        BuoyancyForce {
            gravity: Acceleration::new(0.0, gravity),
            fluid_density: Density::new(fluid_density),
        }
    }
}

impl SimpleInfluenceForce for BuoyancyForce {
    fn calc_force(&self, cell: &Cell, _subtick_duration: Duration) -> Force {
        let displaced_fluid_mass = cell.area() * self.fluid_density;
        let force = -(displaced_fluid_mass * self.gravity);
        trace!("Cell {} Buoyancy {:?}", cell.node_handle(), force);
        force
    }
}

#[derive(Debug)]
pub struct DragForce {
    viscosity: f64,
}

impl DragForce {
    pub fn new(viscosity: f64) -> Self {
        DragForce { viscosity }
    }

    fn calc_drag(
        &self,
        mass: Mass,
        radius: Length,
        velocity: f64,
        subtick_duration: Duration,
    ) -> f64 {
        -velocity.signum()
            * self.instantaneous_abs_drag(radius, velocity).min(
                Self::abs_drag_that_will_stop_the_cell(mass, velocity, subtick_duration),
            )
    }

    fn instantaneous_abs_drag(&self, radius: Length, velocity: f64) -> f64 {
        self.viscosity * radius.value() * sqr(velocity)
    }

    fn abs_drag_that_will_stop_the_cell(
        mass: Mass,
        velocity: f64,
        subtick_duration: Duration,
    ) -> f64 {
        mass.value() * velocity.abs() / subtick_duration.value()
    }
}

impl SimpleInfluenceForce for DragForce {
    fn calc_force(&self, cell: &Cell, subtick_duration: Duration) -> Force {
        let force = Force::new(
            self.calc_drag(
                cell.mass(),
                cell.radius(),
                cell.velocity().x(),
                subtick_duration,
            ),
            self.calc_drag(
                cell.mass(),
                cell.radius(),
                cell.velocity().y(),
                subtick_duration,
            ),
        );
        trace!("Cell {} Drag {:?}", cell.node_handle(), force);
        force
    }
}

#[derive(Debug)]
pub struct UniversalOverlap {
    overlap: Overlap,
}

impl UniversalOverlap {
    pub fn new(overlap: Overlap) -> Self {
        UniversalOverlap { overlap }
    }
}

impl Influence for UniversalOverlap {
    fn apply(
        &self,
        cell_graph: &mut SortableGraph<Cell, Bond, AngleGusset>,
        _subtick_duration: Duration,
    ) {
        for cell in cell_graph.nodes_mut() {
            cell.environment_mut().add_overlap(self.overlap);
        }
    }
}

#[derive(Debug)]
pub struct Sunlight {
    slope: f64,
    intercept: f64,
}

impl Sunlight {
    pub fn new(min_y: f64, max_y: f64, min_intensity: f64, max_intensity: f64) -> Self {
        let slope = (max_intensity - min_intensity) / (max_y - min_y);
        Sunlight {
            slope,
            intercept: max_intensity - slope * max_y,
        }
    }

    fn calc_light_intensity(&self, y: f64) -> f64 {
        (self.slope * y + self.intercept).max(0.0)
    }
}

impl Influence for Sunlight {
    fn apply(
        &self,
        cell_graph: &mut SortableGraph<Cell, Bond, AngleGusset>,
        _subtick_duration: Duration,
    ) {
        for cell in cell_graph.nodes_mut() {
            let y = cell.center().y();
            cell.environment_mut()
                .add_light_intensity(self.calc_light_intensity(y));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biology::layers::*;
    use std::f64::consts::PI;

    #[test]
    fn wall_collisions_add_overlap_and_force_old() {
        let mut cell_graph = SortableGraph::new();
        let wall_collisions = WallCollisions::new(
            Position::new(-10.0, -10.0),
            Position::new(10.0, 10.0),
            Box::new(LinearSpring::new(1.0)),
        );
        let ball_handle = cell_graph.add_node(Cell::ball(
            Length::new(1.0),
            Mass::new(1.0),
            Position::new(9.5, 9.5),
            Velocity::new(1.0, 1.0),
        ));

        wall_collisions.apply(&mut cell_graph, Duration::new(0.5));

        let ball = cell_graph.node(ball_handle);
        assert_eq!(ball.environment().overlaps().len(), 1);
        assert_ne!(ball.forces().net_force().x(), 0.0);
        assert_ne!(ball.forces().net_force().y(), 0.0);
    }

    #[test]
    fn wall_collisions_add_overlap_and_force() {
        let mut cell_graph = SortableGraph::new();
        let wall_collisions = WallCollisions::new(
            Position::new(-10.0, -10.0),
            Position::new(10.0, 10.0),
            Box::new(LinearSpring::new(1.0)),
        );
        let ball_handle = cell_graph.add_node(Cell::ball(
            Length::new(1.0),
            Mass::new(1.0),
            Position::new(9.5, 9.5),
            Velocity::new(1.0, 1.0),
        ));

        wall_collisions.apply(&mut cell_graph, Duration::new(1.0));

        let ball = cell_graph.node(ball_handle);
        assert_eq!(ball.environment().overlaps().len(), 1);
        assert_ne!(ball.forces().net_force().x(), 0.0);
        assert_ne!(ball.forces().net_force().y(), 0.0);
    }

    #[test]
    fn no_walls_collision_force() {
        assert_eq!(
            WallCollisions::collision_force(
                Mass::new(2.0),
                Velocity::new(3.0, 2.0),
                Displacement::new(0.0, 0.0)
            ),
            Force::new(0.0, 0.0)
        );
    }

    #[test]
    fn top_right_walls_fast_collision_force() {
        assert_eq!(
            WallCollisions::collision_force(
                Mass::new(2.0),
                Velocity::new(3.0, 4.0),
                Displacement::new(2.0, 3.0)
            ),
            Force::new(-12.0, -16.0)
        );
    }

    #[test]
    fn top_right_walls_slow_collision_force() {
        assert_eq!(
            WallCollisions::collision_force(
                Mass::new(2.0),
                Velocity::new(1.0, 0.5),
                Displacement::new(2.0, 1.5)
            ),
            Force::new(-6.0, -4.0)
        );
    }

    #[test]
    fn bottom_left_walls_fast_collision_force() {
        assert_eq!(
            WallCollisions::collision_force(
                Mass::new(2.0),
                Velocity::new(-3.0, -4.0),
                Displacement::new(-2.0, -3.0)
            ),
            Force::new(12.0, 16.0)
        );
    }

    #[test]
    fn bottom_left_walls_slow_collision_force() {
        assert_eq!(
            WallCollisions::collision_force(
                Mass::new(2.0),
                Velocity::new(-1.0, -0.5),
                Displacement::new(-2.0, -1.5)
            ),
            Force::new(6.0, 4.0)
        );
    }

    #[test]
    fn pair_collisions_add_overlaps_and_forces() {
        let mut cell_graph = SortableGraph::new();
        let pair_collisions = PairCollisions::new(Box::new(LinearSpring::new(1.0)));
        let ball1_handle = cell_graph.add_node(Cell::ball(
            Length::new(1.0),
            Mass::new(1.0),
            Position::new(0.0, 0.0),
            Velocity::new(1.0, 1.0),
        ));
        let ball2_handle = cell_graph.add_node(Cell::ball(
            Length::new(1.0),
            Mass::new(1.0),
            Position::new(1.4, 1.4),
            Velocity::new(-1.0, -1.0),
        ));

        pair_collisions.apply(&mut cell_graph, Duration::new(0.5));

        let ball1 = cell_graph.node(ball1_handle);
        assert_eq!(ball1.environment().overlaps().len(), 1);
        assert_ne!(ball1.forces().net_force().x(), 0.0);
        assert_ne!(ball1.forces().net_force().y(), 0.0);

        let ball2 = cell_graph.node(ball2_handle);
        assert_eq!(ball2.environment().overlaps().len(), 1);
        assert_ne!(ball2.forces().net_force().x(), 0.0);
        assert_ne!(ball2.forces().net_force().y(), 0.0);
    }

    #[test]
    fn bond_forces_add_forces() {
        let mut cell_graph = SortableGraph::new();
        let bond_forces = BondForces::new();
        let ball1_handle = cell_graph.add_node(Cell::ball(
            Length::new(1.0),
            Mass::new(1.0),
            Position::new(0.0, 0.0),
            Velocity::new(-1.0, -1.0),
        ));
        let ball2_handle = cell_graph.add_node(Cell::ball(
            Length::new(1.0),
            Mass::new(1.0),
            Position::new(1.5, 1.5),
            Velocity::new(1.0, 1.0),
        ));
        let bond = Bond::new(cell_graph.node(ball1_handle), cell_graph.node(ball2_handle));
        cell_graph.add_edge(bond, 1, 0);

        bond_forces.apply(&mut cell_graph, Duration::new(0.5));

        let ball1 = cell_graph.node(ball1_handle);
        assert_ne!(ball1.forces().net_force().x(), 0.0);
        assert_ne!(ball1.forces().net_force().y(), 0.0);

        let ball2 = cell_graph.node(ball2_handle);
        assert_ne!(ball2.forces().net_force().x(), 0.0);
        assert_ne!(ball2.forces().net_force().y(), 0.0);
    }

    #[test]
    fn bond_angle_forces_add_forces() {
        let mut cell_graph = SortableGraph::new();

        let ball1_handle = cell_graph.add_node(Cell::ball(
            Length::new(1.0),
            Mass::new(1.0),
            Position::new(0.1, 2.0),
            Velocity::ZERO,
        ));
        let ball2_handle = cell_graph.add_node(Cell::ball(
            Length::new(1.0),
            Mass::new(1.0),
            Position::new(0.0, 0.0),
            Velocity::ZERO,
        ));
        let ball3_handle = cell_graph.add_node(Cell::ball(
            Length::new(1.0),
            Mass::new(1.0),
            Position::new(0.0, -2.0),
            Velocity::ZERO,
        ));

        let bond = Bond::new(cell_graph.node(ball1_handle), cell_graph.node(ball2_handle));
        let bond1_handle = cell_graph.add_edge(bond, 1, 0);
        let bond = Bond::new(cell_graph.node(ball2_handle), cell_graph.node(ball3_handle));
        let bond2_handle = cell_graph.add_edge(bond, 1, 0);

        let gusset = AngleGusset::new(
            cell_graph.edge(bond1_handle),
            cell_graph.edge(bond2_handle),
            Angle::from_radians(PI),
        );
        cell_graph.add_meta_edge(gusset);

        BondAngleForces::new().apply(&mut cell_graph, Duration::new(0.5));

        let ball3 = cell_graph.node(ball3_handle);
        assert!(ball3.forces().net_force().x() < 0.0);
    }

    #[test]
    fn simple_force_influence_adds_force() {
        let mut cell_graph = SortableGraph::new();
        let force = Force::new(2.0, -3.0);
        let influence = SimpleForceInfluence::new(Box::new(ConstantForce::new(force)));
        let ball_handle = cell_graph.add_node(Cell::ball(
            Length::new(1.0),
            Mass::new(3.0),
            Position::new(0.0, 0.0),
            Velocity::ZERO,
        ));

        influence.apply(&mut cell_graph, Duration::new(0.5));

        let ball = cell_graph.node(ball_handle);
        assert_eq!(ball.forces().net_force(), force);
    }

    #[test]
    fn weight_adds_force_proportional_to_mass() {
        let weight = WeightForce::new(-2.0);
        let ball = Cell::ball(
            Length::new(1.0),
            Mass::new(3.0),
            Position::new(0.0, 0.0),
            Velocity::ZERO,
        );
        assert_eq!(
            weight.calc_force(&ball, Duration::new(0.5)),
            Force::new(0.0, -6.0)
        );
    }

    #[test]
    fn buoyancy_adds_force_proportional_to_area() {
        let buoyancy = BuoyancyForce::new(-2.0, 2.0);
        let ball = Cell::ball(
            Length::new(2.0 / PI.sqrt()),
            Mass::new(1.0),
            Position::new(0.0, 0.0),
            Velocity::ZERO,
        );
        let force = buoyancy.calc_force(&ball, Duration::new(0.5));
        assert_eq!(force.x(), 0.0);
        assert_eq!(force.y().round(), 16.0);
    }

    #[test]
    fn drag_adds_force_proportional_to_radius_and_velocity_squared() {
        let drag = DragForce::new(0.5);
        let ball = Cell::ball(
            Length::new(2.0),
            Mass::new(10.0),
            Position::new(0.0, 0.0),
            Velocity::new(2.0, -3.0),
        );
        assert_eq!(
            drag.calc_force(&ball, Duration::new(0.5)),
            Force::new(-4.0, 9.0)
        );
    }

    #[test]
    fn drag_force_is_limited_to_force_that_will_stop_cell() {
        let drag = DragForce::new(0.5);
        let ball = Cell::ball(
            Length::new(10.0),
            Mass::new(0.01),
            Position::ORIGIN,
            Velocity::new(10.0, -10.0),
        );
        assert_eq!(
            drag.calc_force(&ball, Duration::new(0.5)),
            Force::new(-0.2, 0.2)
        );
    }

    #[test]
    fn sunlight_adds_light() {
        let sunlight = Sunlight::new(-10.0, 10.0, 10.0, 20.0);
        let mut cell_graph = SortableGraph::new();
        let cell_handle = cell_graph.add_node(simple_layered_cell(vec![simple_cell_layer(
            Area::new(PI),
            Density::new(1.0),
        )]));

        sunlight.apply(&mut cell_graph, Duration::new(0.5));

        let cell = cell_graph.node(cell_handle);
        assert_eq!(cell.environment().light_intensity(), 15.0);
    }

    #[test]
    fn sunlight_never_negative() {
        let sunlight = Sunlight::new(-10.0, 0.0, 0.0, 10.0);
        let mut cell_graph = SortableGraph::new();
        let cell_handle = cell_graph.add_node(
            simple_layered_cell(vec![simple_cell_layer(Area::new(1.0), Density::new(1.0))])
                .with_initial_position(Position::new(0.0, -11.0)),
        );

        sunlight.apply(&mut cell_graph, Duration::new(0.5));

        let cell = cell_graph.node(cell_handle);
        assert_eq!(cell.environment().light_intensity(), 0.0);
    }

    fn simple_layered_cell(layers: Vec<CellLayer>) -> Cell {
        Cell::new(Position::ORIGIN, Velocity::ZERO, layers)
    }

    fn simple_cell_layer(area: Area, density: Density) -> CellLayer {
        CellLayer::new(
            area,
            density,
            Color::Green,
            Box::new(NullCellLayerSpecialty::new()),
        )
    }
}
