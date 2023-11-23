use crate::metric::Metric;
use crate::network::Network;
use crate::parameters::interpolate::interpolate;
use crate::parameters::{Parameter, ParameterMeta};
use crate::scenario::ScenarioIndex;
use crate::state::{ParameterState, State};
use crate::timestep::Timestep;
use crate::PywrError;
use std::any::Any;

pub struct PiecewiseInterpolatedParameter {
    meta: ParameterMeta,
    metric: Metric,
    control_curves: Vec<Metric>,
    values: Vec<[f64; 2]>,
    maximum: f64,
    minimum: f64,
}

impl PiecewiseInterpolatedParameter {
    pub fn new(
        name: &str,
        metric: Metric,
        control_curves: Vec<Metric>,
        values: Vec<[f64; 2]>,
        maximum: f64,
        minimum: f64,
    ) -> Self {
        Self {
            meta: ParameterMeta::new(name),
            metric,
            control_curves,
            values,
            maximum,
            minimum,
        }
    }
}

impl Parameter for PiecewiseInterpolatedParameter {
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
        // Current value
        let x = self.metric.get_value(model, state)?;

        let mut cc_previous_value = self.maximum;
        for (idx, control_curve) in self.control_curves.iter().enumerate() {
            let cc_value = control_curve.get_value(model, state)?;
            if x > cc_value {
                let v = self.values.get(idx).ok_or(PywrError::DataOutOfRange)?;
                return Ok(interpolate(x, cc_value, cc_previous_value, v[1], v[0]));
            }
            cc_previous_value = cc_value;
        }
        let v = self.values.last().ok_or(PywrError::DataOutOfRange)?;
        Ok(interpolate(x, self.minimum, cc_previous_value, v[1], v[0]))
    }
}

#[cfg(test)]
mod test {
    use crate::metric::Metric;
    use crate::parameters::{Array1Parameter, PiecewiseInterpolatedParameter};
    use crate::test_utils::{run_and_assert_parameter, simple_model};
    use ndarray::{Array1, Array2, Axis};

    /// Basic functional test of the piecewise interpolation.
    #[test]
    fn test_basic() {
        let mut model = simple_model(1);

        // Create an artificial volume series to use for the interpolation test
        let volume = Array1Parameter::new("test-x", Array1::linspace(1.0, 0.0, 21), None);

        let volume_idx = model.network_mut().add_parameter(Box::new(volume)).unwrap();

        let parameter = PiecewiseInterpolatedParameter::new(
            "test-parameter",
            Metric::ParameterValue(volume_idx), // Interpolate with the parameter based values
            vec![Metric::Constant(0.8), Metric::Constant(0.5)],
            vec![[10.0, 1.0], [0.0, 0.0], [-1.0, -10.0]],
            1.0,
            0.0,
        );

        let expected_values: Array1<f64> = [
            10.0,                    // full
            1.0 + 9.0 * 0.15 / 0.2,  // 95%
            1.0 + 9.0 * 0.10 / 0.2,  // 90%
            1.0 + 9.0 * 0.05 / 0.2,  // 85%
            0.0,                     // 80%
            0.0,                     // 75%
            0.0,                     // 70%
            0.0,                     // 65%
            0.0,                     // 60%
            0.0,                     // 55%
            -1.0,                    // 50%
            -1.0 - 9.0 * 0.05 / 0.5, // 45%
            -1.0 - 9.0 * 0.10 / 0.5, // 40%
            -1.0 - 9.0 * 0.15 / 0.5, // 35%
            -1.0 - 9.0 * 0.20 / 0.5, // 30%
            -1.0 - 9.0 * 0.25 / 0.5, // 25%
            -1.0 - 9.0 * 0.30 / 0.5, // 20%
            -1.0 - 9.0 * 0.35 / 0.5, // 15%
            -1.0 - 9.0 * 0.40 / 0.5, // 10%
            -1.0 - 9.0 * 0.45 / 0.5, // 05%
            -10.0,                   // 00%
        ]
        .to_vec()
        .into();
        let expected_values: Array2<f64> = expected_values.insert_axis(Axis(1));

        run_and_assert_parameter(&mut model, Box::new(parameter), expected_values, None, Some(1e-12));
    }
}