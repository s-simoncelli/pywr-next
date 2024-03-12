use crate::metric::Metric;
use crate::network::Network;
use crate::parameters::{Parameter, ParameterMeta};
use crate::scenario::ScenarioIndex;
use crate::state::{ParameterState, State};
use crate::timestep::Timestep;
use crate::utils::inverse_hydropower_calculation;
use crate::PywrError;
use std::any::Any;

pub struct TurbineData {
    pub target: Metric,
    pub elevation: f64,
    pub min_head: f64,
    pub max_flow: Option<Metric>,
    pub min_flow: Option<Metric>,
    pub efficiency: f64,
    pub water_elevation: Option<Metric>,
    pub water_density: f64,
    pub flow_unit_conversion: f64,
    pub energy_unit_conversion: f64,
}

pub struct HydropowerTargetParameter {
    pub meta: ParameterMeta,
    pub target: Metric,
    pub max_flow: Option<Metric>,
    pub min_flow: Option<Metric>,
    pub turbine_min_head: f64,
    pub turbine_elevation: f64,
    pub turbine_efficiency: f64,
    pub water_elevation: Option<Metric>,
    pub water_density: f64,
    pub flow_unit_conversion: f64,
    pub energy_unit_conversion: f64,
}

impl HydropowerTargetParameter {
    pub fn new(name: &str, turbine_data: TurbineData) -> Self {
        Self {
            meta: ParameterMeta::new(name),
            target: turbine_data.target,
            water_elevation: turbine_data.water_elevation,
            turbine_elevation: turbine_data.elevation.unwrap_or(0.0),
            turbine_min_head: turbine_data.min_head.unwrap_or(0.0),
            turbine_efficiency: turbine_data.efficiency.unwrap_or(1.0),
            max_flow: turbine_data.max_flow,
            min_flow: turbine_data.min_flow,
            water_density: turbine_data.water_density.unwrap_or(1000.0),
            flow_unit_conversion: turbine_data.flow_unit_conversion.unwrap_or(1.0),
            energy_unit_conversion: turbine_data.energy_unit_conversion.unwrap_or(1e-6),
        }
    }
}

impl Parameter for HydropowerTargetParameter {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn meta(&self) -> &ParameterMeta {
        &self.meta
    }
    fn compute(
        &self,
        _timestep: &Timestep,
        _scenario_index: &ScenarioIndex,
        model: &Network,
        state: &State,
        _internal_state: &mut Option<Box<dyn ParameterState>>,
    ) -> Result<f64, PywrError> {
        // Calculate the head
        let mut head = if let Some(water_elevation) = &self.water_elevation {
            water_elevation.get_value(model, state)? - self.turbine_elevation
        } else {
            self.turbine_elevation
        };

        // the head may be negative
        head = head.max(0.0);

        // apply the minimum head threshold
        if head < self.turbine_min_head {
            return Ok(0.0);
        }

        // Get the flow from the current node
        let power = self.target.get_value(model, state)?;
        let mut q = inverse_hydropower_calculation(
            power,
            head,
            0.0,
            self.turbine_efficiency,
            self.flow_unit_conversion,
            self.energy_unit_conversion,
            self.water_density,
        );

        // Bound the flow if required
        if let Some(max_flow) = &self.max_flow {
            q = q.min(max_flow.get_value(model, state)?);
        } else if let Some(min_flow) = &self.min_flow {
            q = q.max(min_flow.get_value(model, state)?);
        }

        if q < 0.0 {
            return Err(PywrError::InternalParameterError(format!(
                "The calculated flow in the hydro power parameter named {} is negative",
                self.name()
            )));
        }
        Ok(q)
    }
}