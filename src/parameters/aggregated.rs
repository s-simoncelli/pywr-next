use super::{NetworkState, PywrError};
use crate::parameters::{FloatValue, Parameter, ParameterMeta};
use crate::scenario::ScenarioIndex;
use crate::state::ParameterState;
use crate::timestep::Timestep;
use std::str::FromStr;

pub enum AggFunc {
    Sum,
    Product,
    Mean,
    Min,
    Max,
}

impl FromStr for AggFunc {
    type Err = PywrError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "sum" => Ok(Self::Sum),
            "product" => Ok(Self::Product),
            "mean" => Ok(Self::Mean),
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            _ => Err(PywrError::InvalidAggregationFunction(name.to_string())),
        }
    }
}

pub struct AggregatedParameter {
    meta: ParameterMeta,
    values: Vec<FloatValue>,
    agg_func: AggFunc,
}

impl AggregatedParameter {
    pub fn new(name: &str, values: Vec<FloatValue>, agg_func: AggFunc) -> Self {
        Self {
            meta: ParameterMeta::new(name),
            values,
            agg_func,
        }
    }
}

impl Parameter for AggregatedParameter {
    fn meta(&self) -> &ParameterMeta {
        &self.meta
    }
    fn compute(
        &mut self,
        _timestep: &Timestep,
        _scenario_index: &ScenarioIndex,
        _state: &NetworkState,
        parameter_state: &ParameterState,
    ) -> Result<f64, PywrError> {
        // TODO scenarios!

        let value: f64 = match self.agg_func {
            AggFunc::Sum => {
                let mut total = 0.0_f64;
                for p in &self.values {
                    total += match p {
                        FloatValue::Constant(v) => *v,
                        FloatValue::Dynamic(p) => parameter_state.get_value(*p)?,
                    };
                }
                total
            }
            AggFunc::Mean => {
                let mut total = 0.0_f64;
                for p in &self.values {
                    total += match p {
                        FloatValue::Constant(v) => *v,
                        FloatValue::Dynamic(p) => parameter_state.get_value(*p)?,
                    };
                }
                total / self.values.len() as f64
            }
            AggFunc::Max => {
                let mut total = f64::MIN;
                for p in &self.values {
                    total = total.max(match p {
                        FloatValue::Constant(v) => *v,
                        FloatValue::Dynamic(p) => parameter_state.get_value(*p)?,
                    });
                }
                total
            }
            AggFunc::Min => {
                let mut total = f64::MAX;
                for p in &self.values {
                    total = total.min(match p {
                        FloatValue::Constant(v) => *v,
                        FloatValue::Dynamic(p) => parameter_state.get_value(*p)?,
                    });
                }
                total
            }
            AggFunc::Product => {
                let mut total = 1.0_f64;
                for p in &self.values {
                    total *= match p {
                        FloatValue::Constant(v) => *v,
                        FloatValue::Dynamic(p) => parameter_state.get_value(*p)?,
                    };
                }
                total
            }
        };

        Ok(value)
    }
}
