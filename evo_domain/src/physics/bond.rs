use crate::physics::quantities::*;
use crate::physics::shapes::*;
use crate::physics::sortable_graph::*;
use crate::physics::util::*;
use evo_domain_derive::*;
use std::f64;
use std::f64::consts::PI;

#[derive(Clone, Debug, GraphEdge, PartialEq)]
pub struct Bond {
    edge_data: GraphEdgeData,
    energy_for_cell1: BioEnergy,
    energy_for_cell2: BioEnergy,
}

impl Bond {
    pub fn new(circle1: &dyn GraphNode, circle2: &dyn GraphNode) -> Self {
        assert_ne!(circle1.node_handle(), circle2.node_handle());
        Bond {
            edge_data: GraphEdgeData::new(circle1.node_handle(), circle2.node_handle()),
            energy_for_cell1: BioEnergy::new(0.0),
            energy_for_cell2: BioEnergy::new(0.0),
        }
    }

    pub fn energy_for_cell1(&self) -> BioEnergy {
        self.energy_for_cell1
    }

    pub fn energy_for_cell2(&self) -> BioEnergy {
        self.energy_for_cell2
    }

    pub fn set_energy_from_cell(&mut self, cell_handle: NodeHandle, energy: BioEnergy) {
        if cell_handle == self.edge_data.node1_handle() {
            self.energy_for_cell2 = energy;
        } else if cell_handle == self.edge_data.node2_handle() {
            self.energy_for_cell1 = energy;
        } else {
            panic!("Tried to set bond energy from an unrelated cell");
        }
    }

    pub fn claim_energy_for_cell(&mut self, cell_handle: NodeHandle) -> BioEnergy {
        let ret_energy;
        if cell_handle == self.edge_data.node1_handle() {
            ret_energy = self.energy_for_cell1;
            self.energy_for_cell1 = BioEnergy::ZERO;
        } else if cell_handle == self.edge_data.node2_handle() {
            ret_energy = self.energy_for_cell2;
            self.energy_for_cell2 = BioEnergy::ZERO;
        } else {
            panic!("Tried to claim bond energy for an unrelated cell");
        }
        ret_energy
    }

    pub fn calc_strain(&self) -> Displacement {
        Displacement::new(0.0, 0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BondStrain {
    strain: Displacement,
}

impl BondStrain {
    pub fn new(strain: Displacement) -> Self {
        BondStrain { strain }
    }

    pub fn strain(&self) -> Displacement {
        self.strain
    }
}

pub fn calc_bond_strains<C>(
    graph: &SortableGraph<C, Bond, AngleGusset>,
) -> Vec<((NodeHandle, BondStrain), (NodeHandle, BondStrain))>
where
    C: Circle + GraphNode,
{
    let mut strains: Vec<((NodeHandle, BondStrain), (NodeHandle, BondStrain))> =
        Vec::with_capacity(graph.edges().len() * 2);
    for bond in graph.edges() {
        let circle1 = graph.node(bond.node1_handle());
        let circle2 = graph.node(bond.node2_handle());

        let strain = calc_bond_strain(circle1, circle2);
        strains.push((
            (circle1.node_handle(), BondStrain::new(strain)),
            (circle2.node_handle(), BondStrain::new(-strain)),
        ));
    }
    strains
}

fn calc_bond_strain<C>(circle1: &C, circle2: &C) -> Displacement
where
    C: Circle,
{
    let x_offset = circle1.center().x() - circle2.center().x();
    let y_offset = circle1.center().y() - circle2.center().y();
    let just_touching_center_sep = circle1.radius().value() + circle2.radius().value();
    let center_sep = (sqr(x_offset) + sqr(y_offset)).sqrt();
    if center_sep == 0.0 {
        return Displacement::new(0.0, 0.0);
    }

    let overlap_mag = just_touching_center_sep - center_sep;
    let x_strain = (x_offset / center_sep) * overlap_mag;
    let y_strain = (y_offset / center_sep) * overlap_mag;
    Displacement::new(x_strain, y_strain)
}

#[derive(Clone, Debug, GraphMetaEdge, PartialEq)]
pub struct AngleGusset {
    meta_edge_data: GraphMetaEdgeData,
    angle: Angle, // counterclockwise angle from bond1 to bond2
}

impl AngleGusset {
    pub fn new(bond1: &Bond, bond2: &Bond, angle: Angle) -> Self {
        assert_ne!(bond1.edge_handle(), bond2.edge_handle());
        assert_eq!(bond1.node2_handle(), bond2.node1_handle());
        AngleGusset {
            meta_edge_data: GraphMetaEdgeData::new(bond1.edge_handle(), bond2.edge_handle()),
            angle,
        }
    }
}

pub fn calc_bond_angle_forces<C>(
    graph: &SortableGraph<C, Bond, AngleGusset>,
) -> Vec<(NodeHandle, Force)>
where
    C: Circle + GraphNode,
{
    let mut forces: Vec<(NodeHandle, Force)> = Vec::with_capacity(graph.meta_edges().len() * 2);
    for gusset in graph.meta_edges() {
        let force_pair = calc_bond_angle_force_pair(gusset, graph);
        forces.push(force_pair.0);
        forces.push(force_pair.1);
    }
    forces
}

fn calc_bond_angle_force_pair<C>(
    gusset: &AngleGusset,
    graph: &SortableGraph<C, Bond, AngleGusset>,
) -> ((NodeHandle, Force), (NodeHandle, Force))
where
    C: Circle + GraphNode,
{
    let bond1 = graph.edge(gusset.edge1_handle());
    let bond2 = graph.edge(gusset.edge2_handle());

    let node1 = graph.node(bond1.node1_handle());
    let node0 = graph.node(bond1.node2_handle());
    let node2 = graph.node(bond2.node2_handle());

    let bond_angle = calc_bond_angle(node0.center(), node1.center(), node2.center());
    let torque = calc_torque_from_angle_deflection(bond_angle - gusset.angle);

    let node1_tangential_force =
        calc_tangential_force_from_torque(node0.center(), node1.center(), torque);
    let node1_force =
        calc_force_from_tangential_force(node0.center(), node1.center(), node1_tangential_force);

    let node2_tangential_force =
        calc_tangential_force_from_torque(node0.center(), node2.center(), -torque);
    let node2_force =
        calc_force_from_tangential_force(node0.center(), node2.center(), node2_tangential_force);

    (
        (node1.node_handle(), node1_force),
        (node2.node_handle(), node2_force),
    )
}

fn calc_bond_angle(origin: Position, point1: Position, point2: Position) -> Angle {
    let angle1 = point1.to_polar_angle(origin);
    let angle2 = point2.to_polar_angle(origin);
    let radians = angle2.radians() - angle1.radians();
    Angle::from_radians(if radians >= 0.0 {
        radians
    } else {
        radians + 2.0 * PI
    })
}

fn calc_torque_from_angle_deflection(deflection: Deflection) -> Torque {
    const SPRING_CONSTANT: f64 = 1.0;
    Torque::new(-deflection.radians() * SPRING_CONSTANT)
}

fn calc_tangential_force_from_torque(origin: Position, point: Position, torque: Torque) -> f64 {
    -torque.value() / point.to_polar_radius(origin).value()
}

fn calc_force_from_tangential_force(
    origin: Position,
    point: Position,
    tangential_force: f64,
) -> Force {
    let force_angle = point.to_polar_angle(origin)
        + Deflection::from_radians(tangential_force.signum() * PI / 2.0);
    Force::new(
        tangential_force.abs() * force_angle.radians().cos(),
        tangential_force.abs() * force_angle.radians().sin(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::simple_graph_elements::*;

    #[test]
    #[should_panic]
    fn cannot_bond_same_ball() {
        let mut graph: SortableGraph<SimpleCircleNode, Bond, AngleGusset> = SortableGraph::new();
        let node = add_simple_circle_node(&mut graph, (0.0, 0.0), 1.0);
        add_bond(&mut graph, node, node);
    }

    #[test]
    fn bond_calculates_strain() {
        // {3, 4, 5} triangle (as {6, 8, 10})
        let circle1 = SimpleCircle::new(Position::new(0.0, 0.0), Length::new(2.0));
        let circle2 = SimpleCircle::new(Position::new(6.0, 8.0), Length::new(3.0));

        let strain = calc_bond_strain(&circle1, &circle2);

        // strain/hypotenuse 5 has legs 3 and 4
        assert_eq!(Displacement::new(3.0, 4.0), strain);
    }

    #[test]
    fn bonded_pair_with_matching_centers_has_no_strain() {
        let circle1 = SimpleCircle::new(Position::new(0.0, 0.0), Length::new(1.0));
        let circle2 = SimpleCircle::new(Position::new(0.0, 0.0), Length::new(1.0));

        let strain = calc_bond_strain(&circle1, &circle2);

        // what else could we do?
        assert_eq!(Displacement::new(0.0, 0.0), strain);
    }

    #[test]
    #[should_panic]
    fn cannot_gusset_same_bond() {
        let mut graph: SortableGraph<SimpleCircleNode, Bond, AngleGusset> = SortableGraph::new();
        let node1 = add_simple_circle_node(&mut graph, (0.0, 0.0), 1.0);
        let node2 = add_simple_circle_node(&mut graph, (2.0, 0.0), 1.0);
        let bond = add_bond(&mut graph, node1, node2);
        add_angle_gusset(&mut graph, bond, bond, PI);
    }

    #[test]
    #[should_panic]
    fn cannot_gusset_unconnected_bonds() {
        let mut graph: SortableGraph<SimpleCircleNode, Bond, AngleGusset> = SortableGraph::new();
        let node1 = add_simple_circle_node(&mut graph, (0.0, 0.0), 1.0);
        let node2 = add_simple_circle_node(&mut graph, (2.0, 0.0), 1.0);
        let node3 = add_simple_circle_node(&mut graph, (10.0, 0.0), 1.0);
        let node4 = add_simple_circle_node(&mut graph, (12.0, 0.0), 1.0);
        let bond1 = add_bond(&mut graph, node1, node2);
        let bond2 = add_bond(&mut graph, node3, node4);
        add_angle_gusset(&mut graph, bond1, bond2, PI);
    }

    #[test]
    fn qualitative_gusset_forces() {
        let mut graph: SortableGraph<SimpleCircleNode, Bond, AngleGusset> = SortableGraph::new();
        let node1 = add_simple_circle_node(&mut graph, (0.1, 2.0), 1.0);
        let node2 = add_simple_circle_node(&mut graph, (0.0, 0.0), 1.0);
        let node3 = add_simple_circle_node(&mut graph, (0.1, -2.0), 1.0);
        let bond1 = add_bond(&mut graph, node1, node2);
        let bond2 = add_bond(&mut graph, node2, node3);
        let gusset = add_angle_gusset(&mut graph, bond1, bond2, PI);

        let force_pair = calc_bond_angle_force_pair(&gusset, &graph);

        assert_eq!(node1, (force_pair.0).0);
        assert!((force_pair.0).1.x() < 0.0);
        assert_eq!(node3, (force_pair.1).0);
        assert!((force_pair.1).1.x() < 0.0);
    }

    #[test]
    fn three_quarter_right_angle_off_origin() {
        let origin = Position::new(1.0, 1.0);
        let angle = calc_bond_angle(origin, Position::new(2.0, 2.0), Position::new(2.0, 0.0));
        assert_eq!(Angle::from_radians(3.0 * PI / 2.0), angle);
    }

    #[test]
    fn angle_wraparound() {
        let origin = Position::new(0.0, 0.0);
        let angle = calc_bond_angle(origin, Position::new(-1.0, 1.0), Position::new(1.0, 1.0));
        assert_eq!(Angle::from_radians(3.0 * PI / 2.0), angle);
    }

    #[test]
    fn calcs_tangential_force_from_torque() {
        let origin = Position::new(1.0, 1.0);
        let tangential_force =
            calc_tangential_force_from_torque(origin, Position::new(3.0, 1.0), -Torque::new(3.0));
        assert_eq!(1.5, tangential_force);
    }

    #[test]
    fn calcs_force_from_tangential_force() {
        let origin = Position::new(1.0, 1.0);
        let force = calc_force_from_tangential_force(origin, Position::new(3.0, 1.0), 1.5);
        assert!(force.x().abs() < 0.00001);
        assert_eq!(1.5, force.y());
    }

    fn add_simple_circle_node(
        graph: &mut SortableGraph<SimpleCircleNode, Bond, AngleGusset>,
        center: (f64, f64),
        radius: f64,
    ) -> NodeHandle {
        graph.add_node(SimpleCircleNode::new(
            Position::new(center.0, center.1),
            Length::new(radius),
        ))
    }

    fn add_bond(
        graph: &mut SortableGraph<SimpleCircleNode, Bond, AngleGusset>,
        node1: NodeHandle,
        node2: NodeHandle,
    ) -> EdgeHandle {
        let bond = Bond::new(graph.node(node1), graph.node(node2));
        graph.add_edge(bond, 1, 0)
    }

    fn add_angle_gusset(
        graph: &mut SortableGraph<SimpleCircleNode, Bond, AngleGusset>,
        bond1: EdgeHandle,
        bond2: EdgeHandle,
        radians: f64,
    ) -> AngleGusset {
        AngleGusset::new(
            graph.edge(bond1),
            graph.edge(bond2),
            Angle::from_radians(radians),
        )
    }
}
