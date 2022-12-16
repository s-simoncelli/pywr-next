use crate::metric::Metric;
use crate::parameters::{Parameter, ParameterMeta};
use crate::scenario::ScenarioIndex;
use crate::state::State;
use crate::timestep::Timestep;
use crate::PywrError;
use std::any::Any;

pub struct ControlCurveParameter {
    meta: ParameterMeta,
    metric: Metric,
    control_curves: Vec<Metric>,
    values: Vec<Metric>,
}

impl ControlCurveParameter {
    pub fn new(name: &str, metric: Metric, control_curves: Vec<Metric>, values: Vec<Metric>) -> Self {
        Self {
            meta: ParameterMeta::new(name),
            metric,
            control_curves,
            values,
        }
    }
}

impl Parameter for ControlCurveParameter {
    fn meta(&self) -> &ParameterMeta {
        &self.meta
    }
    fn compute(
        &self,
        _timestep: &Timestep,
        _scenario_index: &ScenarioIndex,
        state: &State,
        _internal_state: &mut Option<Box<dyn Any + Send>>,
    ) -> Result<f64, PywrError> {
        // Current value
        let x = self.metric.get_value(state)?;

        for (idx, control_curve) in self.control_curves.iter().enumerate() {
            let cc_value = control_curve.get_value(state)?;
            if x >= cc_value {
                let value = self.values.get(idx).ok_or(PywrError::DataOutOfRange)?;
                return value.get_value(state);
            }
        }

        let value = self.values.last().ok_or(PywrError::DataOutOfRange)?;
        value.get_value(state)
    }
}