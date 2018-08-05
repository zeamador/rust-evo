use physics::ball::*;
use physics::sortable_graph::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bond {
    ball1_id: BallId,
    ball2_id: BallId,
    ball1_handle: NodeHandle,
    ball2_handle: NodeHandle,
}

impl Bond {
    pub fn new(ball1: &Ball, ball2: &Ball) -> Self {
        Bond {
            ball1_id: ball1.id(),
            ball2_id: ball2.id(),
            ball1_handle: ball1.handle(),
            ball2_handle: ball2.handle(),
        }
    }

    pub fn ball1<'a>(&self, balls: &'a [Ball]) -> &'a Ball {
        self.ball1_id.ball(balls)
    }

    pub fn ball2<'a>(&self, balls: &'a [Ball]) -> &'a Ball {
        self.ball2_id.ball(balls)
    }
}

impl GraphEdge for Bond {
    fn handle1(&self) -> NodeHandle {
        self.ball1_handle
    }

    fn handle1_mut(&mut self) -> &mut NodeHandle {
        &mut self.ball1_handle
    }

    fn handle2(&self) -> NodeHandle {
        self.ball2_handle
    }

    fn handle2_mut(&mut self) -> &mut NodeHandle {
        &mut self.ball2_handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use physics::quantities::*;

    #[test]
    fn new_bond_has_correct_ball_handles() {
        let mut graph: SortableGraph<Ball, Bond> = SortableGraph::new();

        graph.add_node(Ball::new(Length::new(1.0), Mass::new(1.0),
                                 Position::new(1.0, 1.0), Velocity::new(1.0, 1.0)));
        graph.add_node(Ball::new(Length::new(1.0), Mass::new(1.0),
                                 Position::new(1.0, 1.0), Velocity::new(1.0, 1.0)));

        let ball1 = &graph.nodes()[0];
        let ball2 = &graph.nodes()[1];

        let bond = Bond::new(ball1, ball2);

        assert_eq!(ball1, graph.node(bond.handle1()));
        assert_eq!(ball2, graph.node(bond.handle2()));
    }
}
