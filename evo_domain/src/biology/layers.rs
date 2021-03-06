use crate::biology::changes::*;
use crate::biology::control_requests::*;
use crate::environment::local_environment::LocalEnvironment;
use crate::physics::overlap::Overlap;
use crate::physics::quantities::*;
use std::f64;
use std::f64::consts::PI;
use std::fmt;
use std::fmt::Debug;

// TODO rename as TissueType?
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Color {
    Green,
    White,
    Yellow,
}

#[derive(Debug, Clone, Copy)]
pub struct LayerHealthParameters {
    pub healing_energy_delta: BioEnergyDelta,
    pub entropic_damage_health_delta: f64,
    pub overlap_damage_health_delta: f64,
}

impl LayerHealthParameters {
    pub const DEFAULT: LayerHealthParameters = LayerHealthParameters {
        healing_energy_delta: BioEnergyDelta::ZERO,
        entropic_damage_health_delta: 0.0,
        overlap_damage_health_delta: 0.0,
    };

    fn validate(&self) {
        assert!(self.healing_energy_delta.value() <= 0.0);
        assert!(self.entropic_damage_health_delta <= 0.0);
        assert!(self.overlap_damage_health_delta <= 0.0);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayerResizeParameters {
    pub growth_energy_delta: BioEnergyDelta,
    pub max_growth_rate: f64,
    pub shrinkage_energy_delta: BioEnergyDelta,
    pub max_shrinkage_rate: f64,
}

impl LayerResizeParameters {
    pub const UNLIMITED: LayerResizeParameters = LayerResizeParameters {
        growth_energy_delta: BioEnergyDelta::ZERO,
        max_growth_rate: f64::INFINITY,
        shrinkage_energy_delta: BioEnergyDelta::ZERO,
        max_shrinkage_rate: 1.0,
    };

    fn validate(&self) {
        assert!(self.growth_energy_delta.value() <= 0.0);
        assert!(self.max_growth_rate >= 0.0);
        // self.shrinkage_energy_delta can be negative or positive
        assert!(self.max_shrinkage_rate >= 0.0);
    }
}

#[derive(Debug)]
pub struct CellLayer {
    body: CellLayerBody,
    specialty: Box<dyn CellLayerSpecialty>,
}

impl CellLayer {
    const HEALING_CHANNEL_INDEX: usize = 0;
    const RESIZE_CHANNEL_INDEX: usize = 1;
    const LIVING_BRAIN: LivingCellLayerBrain = LivingCellLayerBrain {};
    const DEAD_BRAIN: DeadCellLayerBrain = DeadCellLayerBrain {};

    pub fn new(
        area: Area,
        density: Density,
        color: Color,
        specialty: Box<dyn CellLayerSpecialty>,
    ) -> Self {
        CellLayer {
            body: CellLayerBody::new(area, density, color),
            specialty,
        }
    }

    pub fn with_health_parameters(
        mut self,
        health_parameters: &'static LayerHealthParameters,
    ) -> Self {
        health_parameters.validate();
        self.body.health_parameters = health_parameters;
        self
    }

    pub fn with_resize_parameters(
        mut self,
        resize_parameters: &'static LayerResizeParameters,
    ) -> Self {
        resize_parameters.validate();
        self.body.resize_parameters = resize_parameters;
        self
    }

    pub fn with_health(mut self, health: f64) -> Self {
        assert!(health >= 0.0);
        self.body.health = health;
        self
    }

    pub fn dead(mut self) -> Self {
        self.damage(1.0);
        self
    }

    pub fn spawn(&self, area: Area) -> Self {
        Self {
            body: self.body.spawn(area),
            specialty: self.specialty.spawn(),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health() > 0.0
    }

    pub fn outer_radius(&self) -> Length {
        self.body.outer_radius
    }

    pub fn color(&self) -> Color {
        self.body.color
    }

    pub fn health(&self) -> f64 {
        self.body.health
    }

    pub fn area(&self) -> Area {
        self.body.area
    }

    pub fn mass(&self) -> Mass {
        self.body.mass
    }

    pub fn damage(&mut self, health_loss: f64) {
        self.body.brain.damage(&mut self.body, health_loss);
    }

    pub fn update_outer_radius(&mut self, inner_radius: Length) {
        self.body.update_outer_radius(inner_radius);
    }

    pub fn after_influences(&mut self, env: &LocalEnvironment) -> (BioEnergy, Force) {
        self.body
            .brain
            .after_influences(&mut *self.specialty, &mut self.body, env)
    }

    pub fn cost_control_request(&mut self, request: ControlRequest) -> CostedControlRequest {
        self.body
            .brain
            .cost_control_request(&mut *self.specialty, &self.body, request)
    }

    pub fn execute_control_request(
        &mut self,
        request: BudgetedControlRequest,
        bond_requests: &mut BondRequests,
        changes: &mut CellChanges,
    ) {
        self.body.brain.execute_control_request(
            &mut *self.specialty,
            &mut self.body,
            request,
            bond_requests,
            changes,
        );
    }

    pub fn reset(&mut self) {
        self.specialty.reset();
    }

    pub fn healing_request(layer_index: usize, delta_health: f64) -> ControlRequest {
        ControlRequest::new(layer_index, Self::HEALING_CHANNEL_INDEX, 0, delta_health)
    }

    pub fn resize_request(layer_index: usize, delta_area: AreaDelta) -> ControlRequest {
        ControlRequest::new(
            layer_index,
            Self::RESIZE_CHANNEL_INDEX,
            0,
            delta_area.value(),
        )
    }

    pub fn apply_changes(&mut self, changes: &CellLayerChanges) {
        self.body.apply_changes(changes);
    }
}

// CellLayerBody is separate from CellLayer so it can be mutably passed to CellLayerSpecialty.
// CellLayerBrain is in CellLayerBody so the brain can change its body to use a new brain.
#[derive(Debug)]
pub struct CellLayerBody {
    area: Area,
    density: Density,
    mass: Mass,
    outer_radius: Length,
    health: f64,
    color: Color,
    brain: &'static dyn CellLayerBrain,
    // TODO move to CellLayerParameters struct?
    health_parameters: &'static LayerHealthParameters,
    resize_parameters: &'static LayerResizeParameters,
}

impl CellLayerBody {
    fn new(area: Area, density: Density, color: Color) -> Self {
        let mut body = CellLayerBody {
            area,
            density,
            mass: Mass::ZERO,
            outer_radius: Length::ZERO,
            health: 1.0,
            color,
            brain: &CellLayer::LIVING_BRAIN,
            health_parameters: &LayerHealthParameters::DEFAULT,
            resize_parameters: &LayerResizeParameters::UNLIMITED,
        };
        body.init_from_area();
        body
    }

    fn spawn(&self, area: Area) -> Self {
        let mut copy = Self {
            area,
            health: 1.0,
            brain: &CellLayer::LIVING_BRAIN,
            ..*self
        };
        copy.init_from_area();
        copy
    }

    fn init_from_area(&mut self) {
        self.mass = self.area * self.density;
        self.outer_radius = (self.area / PI).sqrt();
    }

    fn damage(&mut self, health_loss: f64) {
        assert!(health_loss >= 0.0);
        self.health = (self.health - health_loss).max(0.0);
    }

    fn update_outer_radius(&mut self, inner_radius: Length) {
        self.outer_radius = (inner_radius.sqr() + self.area / PI).sqrt();
    }

    fn cost_restore_health(&self, request: ControlRequest) -> CostedControlRequest {
        CostedControlRequest::unlimited(
            request,
            self.health_parameters.healing_energy_delta
                * self.area.value()
                * request.requested_value(),
        )
    }

    fn cost_resize(&self, request: ControlRequest) -> CostedControlRequest {
        let delta_area = self.bound_resize_delta_area(request.requested_value());
        let energy_delta_per_area = if request.requested_value() >= 0.0 {
            self.resize_parameters.growth_energy_delta
        } else {
            -self.resize_parameters.shrinkage_energy_delta
        };
        CostedControlRequest::limited(request, delta_area, delta_area * energy_delta_per_area)
    }

    fn restore_health(&mut self, delta_health: f64) {
        self.health = self.health + delta_health;
    }

    fn actual_delta_health(&self, requested_delta_health: f64, budgeted_fraction: f64) -> f64 {
        assert!(requested_delta_health >= 0.0);
        (budgeted_fraction * requested_delta_health).min(1.0 - self.health)
    }

    fn resize(&mut self, delta_area: AreaDelta) {
        self.area += delta_area;
        self.mass = self.area * self.density;
    }

    fn actual_delta_area(&self, requested_delta_area: f64, budgeted_fraction: f64) -> AreaDelta {
        let delta_area =
            self.health * budgeted_fraction * self.bound_resize_delta_area(requested_delta_area);
        AreaDelta::new(delta_area.max(-self.area.value()))
    }

    fn bound_resize_delta_area(&self, requested_delta_area: f64) -> f64 {
        if requested_delta_area >= 0.0 {
            // TODO a layer that starts with area 0.0 cannot grow
            let max_delta_area = self.resize_parameters.max_growth_rate * self.area.value();
            requested_delta_area.min(max_delta_area)
        } else {
            let min_delta_area = -self.resize_parameters.max_shrinkage_rate * self.area.value();
            requested_delta_area.max(min_delta_area)
        }
    }

    pub fn apply_changes(&mut self, changes: &CellLayerChanges) {
        self.health += changes.health;
        self.resize(changes.area);
    }
}

trait CellLayerBrain: Debug {
    fn damage(&self, body: &mut CellLayerBody, health_loss: f64);

    fn after_influences(
        &self,
        specialty: &mut dyn CellLayerSpecialty,
        body: &mut CellLayerBody,
        env: &LocalEnvironment,
    ) -> (BioEnergy, Force);

    fn cost_control_request(
        &self,
        specialty: &mut dyn CellLayerSpecialty,
        body: &CellLayerBody,
        request: ControlRequest,
    ) -> CostedControlRequest;

    fn execute_control_request(
        &self,
        specialty: &mut dyn CellLayerSpecialty,
        body: &mut CellLayerBody,
        request: BudgetedControlRequest,
        bond_requests: &mut BondRequests,
        changes: &mut CellChanges,
    );
}

#[derive(Debug)]
struct LivingCellLayerBrain {}

impl LivingCellLayerBrain {
    fn entropic_damage(&self, body: &mut CellLayerBody) {
        let damage = body.health_parameters.entropic_damage_health_delta;
        self.damage(body, -damage);
    }

    fn overlap_damage(&self, body: &mut CellLayerBody, overlaps: &[Overlap]) {
        let overlap_damage = overlaps.iter().fold(0.0, |total_damage, overlap| {
            total_damage + body.health_parameters.overlap_damage_health_delta * overlap.magnitude()
        });
        self.damage(body, -overlap_damage);
    }
}

impl CellLayerBrain for LivingCellLayerBrain {
    fn damage(&self, body: &mut CellLayerBody, health_loss: f64) {
        body.damage(health_loss);
        if body.health == 0.0 {
            body.brain = &CellLayer::DEAD_BRAIN;
        }
    }

    fn after_influences(
        &self,
        specialty: &mut dyn CellLayerSpecialty,
        body: &mut CellLayerBody,
        env: &LocalEnvironment,
    ) -> (BioEnergy, Force) {
        self.entropic_damage(body);
        self.overlap_damage(body, env.overlaps());
        specialty.after_influences(body, env)
    }

    fn cost_control_request(
        &self,
        specialty: &mut dyn CellLayerSpecialty,
        body: &CellLayerBody,
        request: ControlRequest,
    ) -> CostedControlRequest {
        match request.channel_index() {
            CellLayer::HEALING_CHANNEL_INDEX => body.cost_restore_health(request),
            CellLayer::RESIZE_CHANNEL_INDEX => body.cost_resize(request),
            _ => specialty.cost_control_request(request),
        }
    }

    fn execute_control_request(
        &self,
        specialty: &mut dyn CellLayerSpecialty,
        body: &mut CellLayerBody,
        request: BudgetedControlRequest,
        bond_requests: &mut BondRequests,
        changes: &mut CellChanges,
    ) {
        match request.channel_index() {
            CellLayer::HEALING_CHANNEL_INDEX => {
                let delta_health =
                    body.actual_delta_health(request.requested_value(), request.budgeted_fraction());
                body.restore_health(delta_health);

                let layer_changes = &mut changes.layers[request.layer_index()];
                layer_changes.health += delta_health;
                // changes.energy += request.energy_delta() * request.budgeted_fraction();
            }
            CellLayer::RESIZE_CHANNEL_INDEX => {
                let delta_area =
                    body.actual_delta_area(request.requested_value(), request.budgeted_fraction());
                body.resize(delta_area);

                let layer_changes = &mut changes.layers[request.layer_index()];
                layer_changes.area += delta_area;
                changes.energy += request.energy_delta() * request.budgeted_fraction();
            }
            _ => specialty.execute_control_request(body, request, bond_requests),
        }
    }
}

#[derive(Debug)]
struct DeadCellLayerBrain {}

impl CellLayerBrain for DeadCellLayerBrain {
    fn damage(&self, _body: &mut CellLayerBody, _health_loss: f64) {}

    fn after_influences(
        &self,
        _specialty: &mut dyn CellLayerSpecialty,
        _body: &mut CellLayerBody,
        _env: &LocalEnvironment,
    ) -> (BioEnergy, Force) {
        (BioEnergy::ZERO, Force::ZERO)
    }

    fn cost_control_request(
        &self,
        _specialty: &mut dyn CellLayerSpecialty,
        _body: &CellLayerBody,
        request: ControlRequest,
    ) -> CostedControlRequest {
        CostedControlRequest::free(request)
    }

    fn execute_control_request(
        &self,
        _specialty: &mut dyn CellLayerSpecialty,
        _body: &mut CellLayerBody,
        _request: BudgetedControlRequest,
        _bond_requests: &mut BondRequests,
        _changes: &mut CellChanges,
    ) {
    }
}

trait CellLayerSpecialtySpawn {
    fn spawn(&self) -> Box<dyn CellLayerSpecialty>;
}

impl CellLayerSpecialtySpawn for Box<dyn CellLayerSpecialty> {
    fn spawn(&self) -> Box<dyn CellLayerSpecialty> {
        self.box_spawn()
    }
}

pub trait CellLayerSpecialty: Debug {
    fn box_spawn(&self) -> Box<dyn CellLayerSpecialty>;

    fn after_influences(
        &mut self,
        _body: &CellLayerBody,
        _env: &LocalEnvironment,
    ) -> (BioEnergy, Force) {
        (BioEnergy::ZERO, Force::ZERO)
    }

    // TODO implement and use this, e.g. for the invalid-index panic
    //    fn max_control_channel_index(&self) -> usize {
    //        CellLayer::RESIZE_CHANNEL_INDEX
    //    }

    fn cost_control_request(&self, request: ControlRequest) -> CostedControlRequest {
        panic!("Invalid control channel index: {}", request.channel_index());
    }

    fn execute_control_request(
        &mut self,
        _body: &CellLayerBody,
        request: BudgetedControlRequest,
        _bond_requests: &mut BondRequests,
    ) {
        panic!("Invalid control channel index: {}", request.channel_index());
    }

    fn reset(&mut self) {}
}

#[derive(Clone, Copy, Debug)]
pub struct BondRequest {
    pub retain_bond: bool,
    pub budding_angle: Angle,
    pub donation_energy: BioEnergy,
}

impl BondRequest {
    pub const MAX_BONDS: usize = 8;

    pub const NONE: BondRequest = BondRequest {
        retain_bond: false,
        budding_angle: Angle::ZERO,
        donation_energy: BioEnergy::ZERO,
    };

    pub fn reset(&mut self) {
        *self = Self::NONE;
    }
}

impl fmt::Display for BondRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(retain: {}, angle: {:.4}, energy: {:.4})",
            self.retain_bond,
            self.budding_angle.radians(),
            self.donation_energy.value(),
        )
    }
}

pub type BondRequests = [BondRequest; BondRequest::MAX_BONDS];

pub const NONE_BOND_REQUESTS: BondRequests = [BondRequest::NONE; BondRequest::MAX_BONDS];

#[derive(Debug)]
pub struct NullCellLayerSpecialty {}

impl NullCellLayerSpecialty {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        NullCellLayerSpecialty {}
    }
}

impl CellLayerSpecialty for NullCellLayerSpecialty {
    fn box_spawn(&self) -> Box<dyn CellLayerSpecialty> {
        Box::new(NullCellLayerSpecialty::new())
    }
}

#[derive(Debug)]
pub struct ThrusterCellLayerSpecialty {
    force_x: f64,
    force_y: f64,
}

impl ThrusterCellLayerSpecialty {
    const FORCE_X_CHANNEL_INDEX: usize = 2;
    const FORCE_Y_CHANNEL_INDEX: usize = 3;

    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        ThrusterCellLayerSpecialty {
            force_x: 0.0,
            force_y: 0.0,
        }
    }

    pub fn force_x_request(layer_index: usize, value: f64) -> ControlRequest {
        ControlRequest::new(layer_index, Self::FORCE_X_CHANNEL_INDEX, 0, value)
    }

    pub fn force_y_request(layer_index: usize, value: f64) -> ControlRequest {
        ControlRequest::new(layer_index, Self::FORCE_Y_CHANNEL_INDEX, 0, value)
    }
}

impl CellLayerSpecialty for ThrusterCellLayerSpecialty {
    fn box_spawn(&self) -> Box<dyn CellLayerSpecialty> {
        Box::new(ThrusterCellLayerSpecialty::new())
    }

    fn after_influences(
        &mut self,
        _body: &CellLayerBody,
        _env: &LocalEnvironment,
    ) -> (BioEnergy, Force) {
        (BioEnergy::ZERO, Force::new(self.force_x, self.force_y))
    }

    fn cost_control_request(&self, request: ControlRequest) -> CostedControlRequest {
        match request.channel_index() {
            // TODO cost forces based on a parameter struct(?)
            Self::FORCE_X_CHANNEL_INDEX | Self::FORCE_Y_CHANNEL_INDEX => {
                CostedControlRequest::free(request)
            }
            _ => panic!("Invalid control channel index: {}", request.channel_index()),
        }
    }

    fn execute_control_request(
        &mut self,
        body: &CellLayerBody,
        request: BudgetedControlRequest,
        _bond_requests: &mut BondRequests,
    ) {
        match request.channel_index() {
            Self::FORCE_X_CHANNEL_INDEX => {
                self.force_x = body.health * request.budgeted_fraction() * request.requested_value()
            }
            Self::FORCE_Y_CHANNEL_INDEX => {
                self.force_y = body.health * request.budgeted_fraction() * request.requested_value()
            }
            _ => panic!("Invalid control channel index: {}", request.channel_index()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PhotoCellLayerSpecialty {
    efficiency: f64,
}

impl PhotoCellLayerSpecialty {
    pub fn new(efficiency: f64) -> Self {
        PhotoCellLayerSpecialty { efficiency }
    }
}

impl CellLayerSpecialty for PhotoCellLayerSpecialty {
    fn box_spawn(&self) -> Box<dyn CellLayerSpecialty> {
        Box::new(self.clone())
    }

    fn after_influences(
        &mut self,
        body: &CellLayerBody,
        env: &LocalEnvironment,
    ) -> (BioEnergy, Force) {
        (
            BioEnergy::new(
                env.light_intensity() * self.efficiency * body.health * body.area.value(),
            ),
            Force::ZERO,
        )
    }
}

#[derive(Debug)]
pub struct BondingCellLayerSpecialty {}

impl BondingCellLayerSpecialty {
    const RETAIN_BOND_CHANNEL_INDEX: usize = 2;
    const BUDDING_ANGLE_CHANNEL_INDEX: usize = 3;
    const DONATION_ENERGY_CHANNEL_INDEX: usize = 4;

    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        BondingCellLayerSpecialty {}
    }

    pub fn retain_bond_request(
        layer_index: usize,
        bond_index: usize,
        flag: bool,
    ) -> ControlRequest {
        ControlRequest::new(
            layer_index,
            Self::RETAIN_BOND_CHANNEL_INDEX,
            bond_index,
            if flag { 1.0 } else { 0.0 },
        )
    }

    pub fn budding_angle_request(
        layer_index: usize,
        bond_index: usize,
        angle: Angle,
    ) -> ControlRequest {
        ControlRequest::new(
            layer_index,
            Self::BUDDING_ANGLE_CHANNEL_INDEX,
            bond_index,
            angle.radians(),
        )
    }

    pub fn donation_energy_request(
        layer_index: usize,
        bond_index: usize,
        energy: BioEnergy,
    ) -> ControlRequest {
        ControlRequest::new(
            layer_index,
            Self::DONATION_ENERGY_CHANNEL_INDEX,
            bond_index,
            energy.value(),
        )
    }
}

impl CellLayerSpecialty for BondingCellLayerSpecialty {
    fn box_spawn(&self) -> Box<dyn CellLayerSpecialty> {
        Box::new(BondingCellLayerSpecialty::new())
    }

    fn cost_control_request(&self, request: ControlRequest) -> CostedControlRequest {
        match request.channel_index() {
            Self::RETAIN_BOND_CHANNEL_INDEX => CostedControlRequest::free(request),
            Self::BUDDING_ANGLE_CHANNEL_INDEX => CostedControlRequest::free(request),
            Self::DONATION_ENERGY_CHANNEL_INDEX => CostedControlRequest::unlimited(
                request,
                BioEnergyDelta::new(-request.requested_value()),
            ),
            _ => panic!("Invalid control channel index: {}", request.channel_index()),
        }
    }

    fn execute_control_request(
        &mut self,
        body: &CellLayerBody,
        request: BudgetedControlRequest,
        bond_requests: &mut BondRequests,
    ) {
        let bond_request = &mut bond_requests[request.value_index()];
        match request.channel_index() {
            Self::RETAIN_BOND_CHANNEL_INDEX => {
                bond_request.retain_bond = request.requested_value() > 0.0
            }
            Self::BUDDING_ANGLE_CHANNEL_INDEX => {
                bond_request.budding_angle = Angle::from_radians(request.requested_value())
            }
            Self::DONATION_ENERGY_CHANNEL_INDEX => {
                bond_request.donation_energy = body.health
                    * request.budgeted_fraction()
                    * BioEnergy::new(request.requested_value())
            }
            _ => panic!("Invalid control channel index: {}", request.channel_index()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biology::control_requests::BudgetedControlRequest;
    use crate::environment::local_environment::LocalEnvironment;
    use crate::physics::overlap::Overlap;

    #[test]
    fn layer_calculates_mass() {
        let layer = simple_cell_layer(Area::new(2.0 * PI), Density::new(3.0));
        assert_eq!(layer.mass(), Mass::new(6.0 * PI));
    }

    #[test]
    fn single_layer_calculates_outer_radius() {
        let layer = simple_cell_layer(Area::new(4.0 * PI), Density::new(1.0));
        assert_eq!(layer.outer_radius(), Length::new(2.0));
    }

    #[test]
    fn layer_updates_outer_radius_based_on_inner_radius() {
        let mut layer = simple_cell_layer(Area::new(3.0 * PI), Density::new(1.0));
        layer.update_outer_radius(Length::new(1.0));
        assert_eq!(layer.outer_radius(), Length::new(2.0));
    }

    #[test]
    fn layer_resize_updates_area_and_mass() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(2.0));
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted_resize_request(0, 2.0),
            &mut bond_requests,
            &mut changes,
        );
        assert_eq!(layer.area(), Area::new(3.0));
        assert_eq!(layer.mass(), Mass::new(6.0));
        assert_eq!(changes.layers[0].area, AreaDelta::new(2.0));
    }

    #[test]
    fn layer_resize_records_energy_change() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(2.0));
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            budgeted(
                CellLayer::resize_request(0, AreaDelta::new(2.0)),
                BioEnergyDelta::new(10.0),
                0.75,
            ),
            &mut bond_requests,
            &mut changes,
        );
        assert_eq!(changes.energy, BioEnergyDelta::new(7.5));
    }

    #[test]
    fn layer_damage_reduces_health() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0));
        layer.damage(0.25);
        assert_eq!(layer.health(), 0.75);
    }

    #[test]
    fn layer_damage_cannot_reduce_health_below_zero() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0));
        layer.damage(2.0);
        assert_eq!(layer.health(), 0.0);
    }

    #[test]
    fn layer_with_some_health_is_not_dead() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0));
        layer.damage(0.99);
        assert!(layer.is_alive());
    }

    #[test]
    fn layer_with_zero_health_is_dead() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0));
        layer.damage(1.0);
        assert!(!layer.is_alive());
    }

    #[test]
    fn layer_costs_resize_request() {
        const LAYER_RESIZE_PARAMS: LayerResizeParameters = LayerResizeParameters {
            growth_energy_delta: BioEnergyDelta::new(-0.5),
            ..LayerResizeParameters::UNLIMITED
        };

        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0))
            .with_resize_parameters(&LAYER_RESIZE_PARAMS);
        let costed_request =
            layer.cost_control_request(CellLayer::resize_request(0, AreaDelta::new(3.0)));
        assert_eq!(
            costed_request,
            CostedControlRequest::unlimited(
                CellLayer::resize_request(0, AreaDelta::new(3.0)),
                BioEnergyDelta::new(-1.5),
            )
        );
    }

    #[test]
    fn layer_growth_is_limited_by_budgeted_fraction() {
        let mut layer = simple_cell_layer(Area::new(2.0), Density::new(1.0));
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            budgeted(
                CellLayer::resize_request(0, AreaDelta::new(2.0)),
                BioEnergyDelta::ZERO,
                0.5,
            ),
            &mut bond_requests,
            &mut changes,
        );
        assert_eq!(layer.area(), Area::new(3.0));
        assert_eq!(changes.layers[0].area, AreaDelta::new(1.0));
    }

    #[test]
    fn layer_growth_is_limited_by_max_growth_rate() {
        const LAYER_RESIZE_PARAMS: LayerResizeParameters = LayerResizeParameters {
            max_growth_rate: 0.5,
            ..LayerResizeParameters::UNLIMITED
        };

        let mut layer = simple_cell_layer(Area::new(2.0), Density::new(1.0))
            .with_resize_parameters(&LAYER_RESIZE_PARAMS);
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted_resize_request(0, 10.0),
            &mut bond_requests,
            &mut changes,
        );
        assert_eq!(layer.area(), Area::new(3.0));
        assert_eq!(changes.layers[0].area, AreaDelta::new(1.0));
    }

    #[test]
    fn layer_growth_cost_is_limited_by_max_growth_rate() {
        const LAYER_RESIZE_PARAMS: LayerResizeParameters = LayerResizeParameters {
            growth_energy_delta: BioEnergyDelta::new(-3.0),
            max_growth_rate: 0.5,
            ..LayerResizeParameters::UNLIMITED
        };

        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0))
            .with_resize_parameters(&LAYER_RESIZE_PARAMS);
        let control_request = CellLayer::resize_request(0, AreaDelta::new(2.0));
        let costed_request = layer.cost_control_request(control_request);
        assert_eq!(
            costed_request,
            CostedControlRequest::limited(control_request, 0.5, BioEnergyDelta::new(-1.5))
        );
    }

    #[test]
    fn layer_shrinkage_is_limited_by_max_shrinkage_rate() {
        const LAYER_RESIZE_PARAMS: LayerResizeParameters = LayerResizeParameters {
            max_shrinkage_rate: 0.25,
            ..LayerResizeParameters::UNLIMITED
        };

        let mut layer = simple_cell_layer(Area::new(2.0), Density::new(1.0))
            .with_resize_parameters(&LAYER_RESIZE_PARAMS);
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted_resize_request(0, -10.0),
            &mut bond_requests,
            &mut changes,
        );
        assert_eq!(layer.area(), Area::new(1.5));
        assert_eq!(changes.layers[0].area, AreaDelta::new(-0.5));
    }

    #[test]
    fn layer_shrinkage_yield_is_limited_by_max_shrinkage_rate() {
        const LAYER_RESIZE_PARAMS: LayerResizeParameters = LayerResizeParameters {
            shrinkage_energy_delta: BioEnergyDelta::new(3.0),
            max_shrinkage_rate: 0.5,
            ..LayerResizeParameters::UNLIMITED
        };

        let mut layer = simple_cell_layer(Area::new(4.0), Density::new(1.0))
            .with_resize_parameters(&LAYER_RESIZE_PARAMS);
        let control_request = CellLayer::resize_request(0, AreaDelta::new(-10.0));
        let costed_request = layer.cost_control_request(control_request);
        assert_eq!(
            costed_request,
            CostedControlRequest::limited(control_request, -2.0, BioEnergyDelta::new(6.0))
        );
    }

    #[test]
    fn layer_resize_is_reduced_by_reduced_health() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0)).with_health(0.5);
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted_resize_request(0, 10.0),
            &mut bond_requests,
            &mut changes,
        );
        assert_eq!(layer.area(), Area::new(6.0));
        assert_eq!(changes.layers[0].area, AreaDelta::new(5.0));
    }

    #[test]
    fn layer_resize_cost_is_not_reduced_by_reduced_health() {
        const LAYER_RESIZE_PARAMS: LayerResizeParameters = LayerResizeParameters {
            growth_energy_delta: BioEnergyDelta::new(-1.0),
            ..LayerResizeParameters::UNLIMITED
        };

        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0))
            .with_resize_parameters(&LAYER_RESIZE_PARAMS)
            .with_health(0.5);
        let control_request = CellLayer::resize_request(0, AreaDelta::new(1.0));
        let costed_request = layer.cost_control_request(control_request);
        assert_eq!(
            costed_request,
            CostedControlRequest::unlimited(control_request, BioEnergyDelta::new(-1.0))
        );
    }

    #[test]
    fn layer_health_can_be_restored() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0)).with_health(0.5);
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted_healing_request(0, 0.25),
            &mut bond_requests,
            &mut changes,
        );
        assert_eq!(layer.health(), 0.75);
        assert_eq!(changes.layers[0].health, 0.25);
    }

    #[test]
    fn layer_health_cannot_be_restored_above_one() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0)).with_health(0.5);
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted_healing_request(0, 1.0),
            &mut bond_requests,
            &mut changes,
        );
        assert_eq!(layer.health(), 1.0);
        assert_eq!(changes.layers[0].health, 0.5);
    }

    #[test]
    fn layer_health_restoration_is_limited_by_budgeted_fraction() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0)).with_health(0.5);
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            budgeted(
                CellLayer::healing_request(0, 0.5),
                BioEnergyDelta::ZERO,
                0.5,
            ),
            &mut bond_requests,
            &mut changes,
        );
        assert_eq!(layer.health(), 0.75);
        assert_eq!(changes.layers[0].health, 0.25);
    }

    #[test]
    fn layer_costs_health_restoration() {
        const LAYER_HEALTH_PARAMS: LayerHealthParameters = LayerHealthParameters {
            healing_energy_delta: BioEnergyDelta::new(-3.0),
            ..LayerHealthParameters::DEFAULT
        };

        let mut layer = simple_cell_layer(Area::new(2.0), Density::new(1.0))
            .with_health_parameters(&LAYER_HEALTH_PARAMS)
            .with_health(0.5);
        let control_request = CellLayer::healing_request(0, 0.25);
        let costed_request = layer.cost_control_request(control_request);
        assert_eq!(
            costed_request,
            CostedControlRequest::unlimited(control_request, BioEnergyDelta::new(-1.5))
        );
    }

    #[test]
    fn layer_undergoes_entropic_damage() {
        const LAYER_HEALTH_PARAMS: LayerHealthParameters = LayerHealthParameters {
            entropic_damage_health_delta: -0.25,
            ..LayerHealthParameters::DEFAULT
        };

        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0))
            .with_health_parameters(&LAYER_HEALTH_PARAMS);

        let env = LocalEnvironment::new();
        layer.after_influences(&env);

        assert_eq!(layer.health(), 0.75);
    }

    #[test]
    fn overlap_damages_layer() {
        const LAYER_HEALTH_PARAMS: LayerHealthParameters = LayerHealthParameters {
            overlap_damage_health_delta: -0.25,
            ..LayerHealthParameters::DEFAULT
        };

        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0))
            .with_health_parameters(&LAYER_HEALTH_PARAMS);

        let mut env = LocalEnvironment::new();
        env.add_overlap(Overlap::new(Displacement::new(0.5, 0.0), 1.0));
        layer.after_influences(&env);

        assert_eq!(layer.health(), 0.875);
    }

    #[test]
    fn dead_layer_costs_control_requests_at_zero() {
        const LAYER_HEALTH_PARAMS: LayerHealthParameters = LayerHealthParameters {
            healing_energy_delta: BioEnergyDelta::new(-1.0),
            ..LayerHealthParameters::DEFAULT
        };

        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0))
            .with_health_parameters(&LAYER_HEALTH_PARAMS)
            .dead();
        let control_request = CellLayer::healing_request(0, 1.0);
        let costed_request = layer.cost_control_request(control_request);
        assert_eq!(costed_request, CostedControlRequest::free(control_request));
    }

    #[test]
    fn dead_layer_ignores_control_requests() {
        let mut layer = simple_cell_layer(Area::new(1.0), Density::new(1.0)).dead();
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted_healing_request(0, 1.0),
            &mut bond_requests,
            &mut changes,
        );
        assert_eq!(layer.health(), 0.0);
    }

    #[test]
    fn thruster_layer_adds_force() {
        let mut layer = CellLayer::new(
            Area::new(1.0),
            Density::new(1.0),
            Color::Green,
            Box::new(ThrusterCellLayerSpecialty::new()),
        );
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted(ThrusterCellLayerSpecialty::force_x_request(0, 1.0)),
            &mut bond_requests,
            &mut changes,
        );
        layer.execute_control_request(
            fully_budgeted(ThrusterCellLayerSpecialty::force_y_request(0, -1.0)),
            &mut bond_requests,
            &mut changes,
        );

        let env = LocalEnvironment::new();
        let (_, force) = layer.after_influences(&env);

        assert_eq!(force, Force::new(1.0, -1.0));
    }

    #[test]
    fn thruster_layer_force_is_limited_by_budget() {
        let mut layer = CellLayer::new(
            Area::new(1.0),
            Density::new(1.0),
            Color::Green,
            Box::new(ThrusterCellLayerSpecialty::new()),
        );
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            budgeted(
                ThrusterCellLayerSpecialty::force_x_request(0, 1.0),
                BioEnergyDelta::new(1.0),
                0.5,
            ),
            &mut bond_requests,
            &mut changes,
        );
        layer.execute_control_request(
            budgeted(
                ThrusterCellLayerSpecialty::force_y_request(0, -1.0),
                BioEnergyDelta::new(1.0),
                0.25,
            ),
            &mut bond_requests,
            &mut changes,
        );

        let env = LocalEnvironment::new();
        let (_, force) = layer.after_influences(&env);

        assert_eq!(force, Force::new(0.5, -0.25));
    }

    #[test]
    fn thruster_layer_force_is_limited_by_health() {
        let mut layer = CellLayer::new(
            Area::new(1.0),
            Density::new(1.0),
            Color::Green,
            Box::new(ThrusterCellLayerSpecialty::new()),
        )
        .with_health(0.5);
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted(ThrusterCellLayerSpecialty::force_x_request(0, 1.0)),
            &mut bond_requests,
            &mut changes,
        );
        layer.execute_control_request(
            fully_budgeted(ThrusterCellLayerSpecialty::force_y_request(0, -1.0)),
            &mut bond_requests,
            &mut changes,
        );

        let env = LocalEnvironment::new();
        let (_, force) = layer.after_influences(&env);

        assert_eq!(force, Force::new(0.5, -0.5));
    }

    #[test]
    fn dead_thruster_layer_adds_no_force() {
        let mut layer = CellLayer::new(
            Area::new(1.0),
            Density::new(1.0),
            Color::Green,
            Box::new(ThrusterCellLayerSpecialty::new()),
        );
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted(ThrusterCellLayerSpecialty::force_x_request(0, 1.0)),
            &mut bond_requests,
            &mut changes,
        );
        layer.execute_control_request(
            fully_budgeted(ThrusterCellLayerSpecialty::force_y_request(0, -1.0)),
            &mut bond_requests,
            &mut changes,
        );
        layer.damage(1.0);

        let env = LocalEnvironment::new();
        let (_, force) = layer.after_influences(&env);

        assert_eq!(force, Force::new(0.0, 0.0));
    }

    #[test]
    fn photo_layer_adds_energy_based_on_area_and_efficiency_and_duration() {
        let mut layer = CellLayer::new(
            Area::new(4.0),
            Density::new(1.0),
            Color::Green,
            Box::new(PhotoCellLayerSpecialty::new(0.5)),
        );

        let mut env = LocalEnvironment::new();
        env.add_light_intensity(10.0);

        let (energy, _) = layer.after_influences(&env);

        assert_eq!(energy, BioEnergy::new(20.0));
    }

    #[test]
    fn photo_layer_energy_is_limited_by_health() {
        let mut layer = CellLayer::new(
            Area::new(1.0),
            Density::new(1.0),
            Color::Green,
            Box::new(PhotoCellLayerSpecialty::new(1.0)),
        )
        .with_health(0.75);

        let mut env = LocalEnvironment::new();
        env.add_light_intensity(1.0);

        let (energy, _) = layer.after_influences(&env);

        assert_eq!(energy, BioEnergy::new(0.75));
    }

    #[test]
    fn dead_photo_layer_adds_no_energy() {
        let mut layer = CellLayer::new(
            Area::new(1.0),
            Density::new(1.0),
            Color::Green,
            Box::new(PhotoCellLayerSpecialty::new(1.0)),
        )
        .dead();

        let mut env = LocalEnvironment::new();
        env.add_light_intensity(1.0);

        let (energy, _) = layer.after_influences(&env);

        assert_eq!(energy, BioEnergy::new(0.0));
    }

    #[test]
    fn budding_energy_is_limited_by_budget() {
        let mut layer = CellLayer::new(
            Area::new(1.0),
            Density::new(1.0),
            Color::Green,
            Box::new(BondingCellLayerSpecialty::new()),
        );
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            budgeted(
                BondingCellLayerSpecialty::donation_energy_request(0, 0, BioEnergy::new(1.0)),
                BioEnergyDelta::new(1.0),
                0.5,
            ),
            &mut bond_requests,
            &mut changes,
        );

        assert_eq!(bond_requests[0].donation_energy, BioEnergy::new(0.5));
    }

    #[test]
    fn budding_energy_is_limited_by_health() {
        let mut layer = CellLayer::new(
            Area::new(1.0),
            Density::new(1.0),
            Color::Green,
            Box::new(BondingCellLayerSpecialty::new()),
        )
        .with_health(0.5);
        let mut bond_requests = NONE_BOND_REQUESTS;
        let mut changes = CellChanges::new(1);
        layer.execute_control_request(
            fully_budgeted(BondingCellLayerSpecialty::donation_energy_request(
                0,
                0,
                BioEnergy::new(1.0),
            )),
            &mut bond_requests,
            &mut changes,
        );

        assert_eq!(bond_requests[0].donation_energy, BioEnergy::new(0.5));
    }

    fn simple_cell_layer(area: Area, density: Density) -> CellLayer {
        CellLayer::new(
            area,
            density,
            Color::Green,
            Box::new(NullCellLayerSpecialty::new()),
        )
    }

    fn fully_budgeted_healing_request(layer_index: usize, value: f64) -> BudgetedControlRequest {
        fully_budgeted(CellLayer::healing_request(layer_index, value))
    }

    fn fully_budgeted_resize_request(layer_index: usize, value: f64) -> BudgetedControlRequest {
        fully_budgeted(CellLayer::resize_request(
            layer_index,
            AreaDelta::new(value),
        ))
    }

    fn fully_budgeted(control_request: ControlRequest) -> BudgetedControlRequest {
        budgeted(control_request, BioEnergyDelta::ZERO, 1.0)
    }

    fn budgeted(
        control_request: ControlRequest,
        cost: BioEnergyDelta,
        budgeted_fraction: f64,
    ) -> BudgetedControlRequest {
        BudgetedControlRequest::new(
            CostedControlRequest::unlimited(control_request, cost),
            budgeted_fraction,
        )
    }
}
