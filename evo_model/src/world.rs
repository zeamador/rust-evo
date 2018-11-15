use environment::environment::*;
use environment::influences::*;
use physics::bond::*;
use physics::newtonian::NewtonianBody;
use physics::quantities::*;
use physics::shapes::*;
use physics::sortable_graph::*;
use physics::spring::*;

pub struct World<C>
    where C: Circle + GraphNode + HasLocalEnvironment + NewtonianBody + Onion
{
    min_corner: Position,
    max_corner: Position,
    cell_graph: SortableGraph<C, Bond, AngleGusset>,
    influences: Vec<Box<Influence<C>>>,
}

impl<C> World<C>
    where C: Circle + GraphNode + HasLocalEnvironment + NewtonianBody + Onion
{
    pub fn new(min_corner: Position, max_corner: Position) -> Self {
        World {
            min_corner,
            max_corner,
            cell_graph: SortableGraph::new(),
            influences: vec![],
        }
    }

    pub fn with_standard_influences(self) -> Self {
        self.with_perimeter_walls()
            .with_influences(vec![
                Box::new(PairCollisions::new()),
                Box::new(BondForces::new()),
                Box::new(BondAngleForces::new()),
            ])
    }

    pub fn with_perimeter_walls(self) -> Self {
        let world_min_corner = self.min_corner();
        let world_max_corner = self.max_corner();
        self.with_influence(Box::new(
            WallCollisions::new(world_min_corner, world_max_corner,
                                Box::new(LinearSpring::new(1.0)))))
    }

    pub fn with_influence(mut self, influence: Box<Influence<C>>) -> Self {
        self.influences.push(influence);
        self
    }

    pub fn with_influences(mut self, mut influences: Vec<Box<Influence<C>>>) -> Self {
        self.influences.append(&mut influences);
        self
    }

    pub fn min_corner(&self) -> Position {
        self.min_corner
    }

    pub fn max_corner(&self) -> Position {
        self.max_corner
    }

    pub fn with_cell(mut self, cell: C) -> Self {
        self.add_cell(cell);
        self
    }

    pub fn with_cells(mut self, cells: Vec<C>) -> Self {
        for cell in cells {
            self.add_cell(cell);
        }
        self
    }

    pub fn add_cell(&mut self, cell: C) {
        self.cell_graph.add_node(cell);
    }

    pub fn cells(&self) -> &[C] {
        &self.cell_graph.unsorted_nodes()
    }

    pub fn with_bonds(mut self, index_pairs: Vec<(usize, usize)>) -> Self {
        for pair in index_pairs {
            let bond = Bond::new(&self.cells()[pair.0], &self.cells()[pair.1]);
            self.add_bond(bond);
        }
        self
    }

    pub fn add_bond(&mut self, bond: Bond) {
        self.cell_graph.add_edge(bond);
    }

    pub fn bonds(&self) -> &[Bond] {
        &self.cell_graph.edges()
    }

    pub fn with_angle_gussets(mut self, index_pairs_with_angles: Vec<(usize, usize, f64)>) -> Self {
        for tuple in index_pairs_with_angles {
            let gusset = AngleGusset::new(&self.bonds()[tuple.0], &self.bonds()[tuple.1], Angle::from_radians(tuple.2));
            self.add_angle_gusset(gusset);
        }
        self
    }

    pub fn add_angle_gusset(&mut self, gusset: AngleGusset) {
        self.cell_graph.add_meta_edge(gusset);
    }

    pub fn tick(&mut self) {
        let tick_duration = Duration::new(1.0);
        let subticks_per_tick = 2;
        let subtick_duration = tick_duration / (subticks_per_tick as f64);

        for _subtick in 0..subticks_per_tick {
            self.apply_influences();
            self.exert_forces(subtick_duration);
            self.clear_influences();
        }
    }

    fn apply_influences(&mut self) {
        for influence in &self.influences {
            influence.apply(&mut self.cell_graph);
        }
    }

    fn exert_forces(&mut self, subtick_duration: Duration) {
        for cell in self.cell_graph.unsorted_nodes_mut() {
            cell.exert_forces(subtick_duration);
            cell.move_for(subtick_duration);
        }
    }

    fn clear_influences(&mut self) -> () {
        for cell in self.cell_graph.unsorted_nodes_mut() {
            cell.environment_mut().clear();
            cell.forces_mut().clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use physics::ball::Ball;
    use physics::overlap::Overlap;

    #[test]
    fn tick_moves_ball() {
        let mut world = create_world(vec![]);
        world.add_cell(Ball::new(Length::new(1.0), Mass::new(1.0),
                                 Position::new(0.0, 0.0), Velocity::new(1.0, 1.0)));
        world.tick();
        let ball = &world.cells()[0];
        assert!(ball.position().x() > 0.0);
        assert!(ball.position().y() > 0.0);
    }

    #[test]
    fn tick_with_force_accelerates_ball() {
        let mut world = create_world(vec![
            Box::new(SimpleForceInfluence::new(Box::new(ConstantForce::new(Force::new(1.0, 1.0)))))
        ]);
        world.add_cell(Ball::new(Length::new(1.0), Mass::new(1.0),
                                 Position::new(0.0, 0.0), Velocity::new(0.0, 0.0)));
        world.tick();
        let ball = &world.cells()[0];
        assert!(ball.velocity().x() > 0.0);
        assert!(ball.velocity().y() > 0.0);
    }

    #[test]
    fn overlaps_do_not_persist() {
        let mut world = create_world(vec![
            Box::new(UniversalOverlap::new(Overlap::new(Displacement::new(1.0, 1.0))))
        ]);
        world.add_cell(Ball::new(Length::new(1.0), Mass::new(1.0),
                                 Position::new(0.0, 0.0), Velocity::new(0.0, 0.0)));
        world.tick();
        let ball = &world.cells()[0];
        assert!(ball.environment().overlaps().is_empty());
    }

    #[test]
    fn forces_do_not_persist() {
        let mut world = create_world(vec![
            Box::new(SimpleForceInfluence::new(Box::new(ConstantForce::new(Force::new(1.0, 1.0)))))
        ]);
        world.add_cell(Ball::new(Length::new(1.0), Mass::new(1.0),
                                 Position::new(0.0, 0.0), Velocity::new(0.0, 0.0)));
        world.tick();
        let ball = &world.cells()[0];
        assert_eq!(Force::new(0.0, 0.0), ball.forces().net_force());
    }

    fn create_world(influences: Vec<Box<Influence<Ball>>>) -> World<Ball> {
        let world = World::new(Position::new(0.0, 0.0), Position::new(0.0, 0.0));
        world.with_influences(influences)
    }
}
