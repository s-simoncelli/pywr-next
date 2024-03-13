use crate::data_tables::LoadedTableCollection;
use crate::model::PywrMultiNetworkTransfer;
use crate::nodes::{NodeAttribute, NodeMeta};
use crate::parameters::DynamicFloatValue;
use crate::SchemaError;
use pywr_core::derived_metric::{DerivedMetric, TurbineData};
use pywr_core::metric::Metric;
use pywr_core::models::ModelDomain;
use pywr_core::parameters::HydropowerTargetData;
use std::collections::HashMap;
use std::path::Path;

/// This turbine node can be used to set a flow constraint based on a hydropower production target.
/// The turbine elevation, minimum head and efficiency can also be configured.
#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct TurbineNode {
    #[serde(flatten)]
    pub meta: NodeMeta,
    pub max_flow: Option<DynamicFloatValue>,
    pub min_flow: Option<DynamicFloatValue>,
    pub cost: Option<DynamicFloatValue>,
    /// Hydropower production target. If set the node's max flow is limited to the flow
    /// calculated using the hydropower. equation. If omitted no flow restriction is set.
    /// Units should be in units of energy per day.
    pub target: Option<DynamicFloatValue>,
    /// The elevation of water entering the turbine. The difference of this value with the
    /// `turbine_elevation` gives the working head of the turbine. This is optional
    /// and can be a constant, a value from a table, a parameter name or an inline parameter
    /// (see [`DynamicFloatValue`]).
    pub water_elevation: Option<DynamicFloatValue>,
    /// The elevation of the turbine. The difference between the `water_elevation` and this value
    /// gives the working head of the turbine. Default to `0.0`.
    pub turbine_elevation: f64,
    /// The minimum head for flow to occur. If the working head is less than this value, zero flow
    /// is returned. Default to `0.0`.
    pub min_head: f64,
    /// The efficiency of the turbine. Default to `1.0`.
    pub efficiency: f64,
    /// The density of water. Default to `1000.0`.
    pub water_density: f64,
    /// A factor used to transform the units of flow to be compatible with the hydropower equation.
    /// This should convert flow to units of m<sup>3</sup> day<sup>-1</sup>. Default to `1.0`.
    pub flow_unit_conversion: f64,
    /// A factor used to transform the units of total energy. Defaults to 1e<sup>-6</sup> to
    /// return `MJ`.
    pub energy_unit_conversion: f64,
}

impl Default for TurbineNode {
    fn default() -> Self {
        Self {
            target: None,
            water_elevation: None,
            turbine_elevation: 0.0,
            min_head: 0.0,
            efficiency: 1.0,
            water_density: 1000.0,
            flow_unit_conversion: 1.0,
            energy_unit_conversion: 1e-6,
            ..Default::default()
        }
    }
}

impl TurbineNode {
    const DEFAULT_ATTRIBUTE: NodeAttribute = NodeAttribute::Outflow;

    pub fn parameters(&self) -> HashMap<&str, &DynamicFloatValue> {
        let mut attributes = HashMap::new();
        if let Some(p) = &self.cost {
            attributes.insert("cost", p);
        }

        attributes
    }

    fn sub_name() -> Option<&'static str> {
        Some("turbine")
    }

    pub fn add_to_model(&self, network: &mut pywr_core::network::Network) -> Result<(), SchemaError> {
        network.add_link_node(self.meta.name.as_str(), None)?;
        Ok(())
    }

    pub fn set_constraints(
        &self,
        network: &mut pywr_core::network::Network,
        schema: &crate::model::PywrNetwork,
        domain: &ModelDomain,
        tables: &LoadedTableCollection,
        data_path: Option<&Path>,
        inter_network_transfers: &[PywrMultiNetworkTransfer],
    ) -> Result<(), SchemaError> {
        if let Some(cost) = &self.cost {
            let value = cost.load(network, schema, domain, tables, data_path, inter_network_transfers)?;
            network.set_node_cost(self.meta.name.as_str(), None, value.into())?;
        }

        if let Some(target) = &self.target {
            // TODO: address parameter name. See https://github.com/pywr/pywr-next/issues/107#issuecomment-1980957962
            let name = format!("{}-power", self.meta.name.as_str());
            let target_value = target.load(network, schema, domain, tables, data_path, inter_network_transfers)?;

            let water_elevation = self
                .water_elevation
                .map(|t| t.load(network, schema, domain, tables, data_path, inter_network_transfers))
                .transpose()?;
            let max_flow = self
                .max_flow
                .map(|t| t.load(network, schema, domain, tables, data_path, inter_network_transfers))
                .transpose()?;
            let min_flow = self
                .min_flow
                .map(|t| t.load(network, schema, domain, tables, data_path, inter_network_transfers))
                .transpose()?;

            let turbine_data = HydropowerTargetData {
                target: target_value,
                water_elevation,
                elevation: Some(self.turbine_elevation),
                min_head: Some(self.min_head),
                max_flow,
                min_flow,
                efficiency: Some(self.efficiency),
                water_density: Some(self.water_density),
                flow_unit_conversion: Some(self.flow_unit_conversion),
                energy_unit_conversion: Some(self.energy_unit_conversion),
            };
            let p = pywr_core::parameters::HydropowerTargetParameter::new(&name, turbine_data);
            let power_idx = network.add_parameter(Box::new(p))?;
            let metric = Metric::ParameterValue(power_idx);
            network.set_node_max_flow(self.meta.name.as_str(), Self::sub_name(), metric.clone().into())?;
        }

        Ok(())
    }

    pub fn input_connectors(&self) -> Vec<(&str, Option<String>)> {
        vec![(self.meta.name.as_str(), None)]
    }
    pub fn output_connectors(&self) -> Vec<(&str, Option<String>)> {
        vec![(self.meta.name.as_str(), None)]
    }

    pub fn create_metric(
        &self,
        network: &mut pywr_core::network::Network,
        attribute: Option<NodeAttribute>,
    ) -> Result<Metric, SchemaError> {
        // Use the default attribute if none is specified
        let attr = attribute.unwrap_or(Self::DEFAULT_ATTRIBUTE);

        let idx = network.get_node_index_by_name(self.meta.name.as_str(), None)?;

        let metric = match attr {
            NodeAttribute::Outflow => Metric::NodeOutFlow(idx),
            NodeAttribute::Inflow => Metric::NodeInFlow(idx),
            NodeAttribute::Power => {
                // TODO
                let water_elevation = self
                    .water_elevation
                    .map(|t| t.load(network, schema, domain, tables, data_path, inter_network_transfers))
                    .transpose()?;

                let turbine_data = TurbineData {
                    elevation: self.turbine_elevation,
                    efficiency: self.efficiency,
                    water_elevation,
                    water_density: self.water_density,
                    flow_unit_conversion: self.flow_unit_conversion,
                    energy_unit_conversion: self.energy_unit_conversion,
                };
                let dm = DerivedMetric::PowerFromNodeFlow(idx, turbine_data);
                let dm_idx = network.add_derived_metric(dm);
                Metric::DerivedMetric(dm_idx)
            }
            _ => {
                return Err(SchemaError::NodeAttributeNotSupported {
                    ty: "TurbineNode".to_string(),
                    name: self.meta.name.clone(),
                    attr,
                })
            }
        };

        Ok(metric)
    }
}