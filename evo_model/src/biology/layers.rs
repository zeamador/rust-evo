use physics::quantities::*;
use std::f64::consts::PI;

#[derive(Debug)]
pub struct SimpleCellLayer {
    area: Area,
    density: Density,
    mass: Mass,
    inner_radius: Length,
    outer_radius: Length,
}

impl SimpleCellLayer {
    pub fn new(area: Area, density: Density) -> Self {
        SimpleCellLayer {
            area,
            density,
            mass: area * density,
            inner_radius: Length::new(0.0),
            outer_radius: Length::new((area.value() / PI).sqrt()),
        }
    }

    pub fn new_old(outer_radius: Length, density: Density) -> Self {
        let area = PI * outer_radius * outer_radius;
        SimpleCellLayer {
            area,
            density,
            mass: area * density,
            inner_radius: Length::new(0.0),
            outer_radius,
        }
    }

    pub fn area(&self) -> Area {
        self.area
    }

    pub fn density(&self) -> Density {
        self.density
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }

    pub fn inner_radius(&self) -> Length {
        self.inner_radius
    }

    pub fn outer_radius(&self) -> Length {
        self.outer_radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_calculates_mass() {
        let layer = SimpleCellLayer::new(Area::new(2.0 * PI), Density::new(3.0));
        assert_eq!(Mass::new(6.0 * PI), layer.mass());
    }

    #[test]
    fn single_layer_calculates_outer_radius() {
        let layer = SimpleCellLayer::new(Area::new(4.0 * PI), Density::new(1.0));
        assert_eq!(Length::new(2.0), layer.outer_radius());
    }
}
