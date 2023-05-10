use crate::node::{ConstraintValue, StorageInitialVolume};
use crate::schema::data_tables::LoadedTableCollection;
use crate::schema::error::ConversionError;
use crate::schema::nodes::NodeMeta;
use crate::schema::parameters::{DynamicFloatValue, TryIntoV2Parameter};
use crate::virtual_storage::VirtualStorageReset;
use crate::PywrError;
use pywr_schema::nodes::AnnualVirtualStorageNode as AnnualVirtualStorageNodeV1;
use std::path::Path;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct AnnualReset {
    pub day: u8,
    pub month: time::Month,
    pub use_initial_volume: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct AnnualVirtualStorageNode {
    #[serde(flatten)]
    pub meta: NodeMeta,
    pub nodes: Vec<String>,
    pub factors: Option<Vec<f64>>,
    pub max_volume: Option<DynamicFloatValue>,
    pub min_volume: Option<DynamicFloatValue>,
    pub cost: Option<DynamicFloatValue>,
    pub initial_volume: Option<f64>,
    pub initial_volume_pc: Option<f64>,
    pub reset: AnnualReset,
}

impl AnnualVirtualStorageNode {
    pub fn add_to_model(
        &self,
        model: &mut crate::model::Model,
        tables: &LoadedTableCollection,
        data_path: Option<&Path>,
    ) -> Result<(), PywrError> {
        let initial_volume = if let Some(iv) = self.initial_volume {
            StorageInitialVolume::Absolute(iv)
        } else if let Some(pc) = self.initial_volume_pc {
            StorageInitialVolume::Proportional(pc)
        } else {
            return Err(PywrError::MissingInitialVolume(self.meta.name.to_string()));
        };

        let min_volume = match &self.min_volume {
            Some(v) => v.load(model, tables, data_path)?.into(),
            None => ConstraintValue::Scalar(0.0),
        };

        let max_volume = match &self.max_volume {
            Some(v) => v.load(model, tables, data_path)?.into(),
            None => ConstraintValue::None,
        };

        let node_idxs = self
            .nodes
            .iter()
            .map(|name| model.get_node_index_by_name(name.as_str(), None))
            .collect::<Result<Vec<_>, _>>()?;

        let reset = VirtualStorageReset::DayOfYear {
            day: self.reset.day,
            month: self.reset.month,
        };

        model.add_virtual_storage_node(
            self.meta.name.as_str(),
            None,
            node_idxs.as_ref(),
            self.factors.as_deref(),
            initial_volume,
            min_volume,
            max_volume,
            reset,
        )?;
        Ok(())
    }

    pub fn input_connectors(&self) -> Vec<(&str, Option<String>)> {
        vec![]
    }

    pub fn output_connectors(&self) -> Vec<(&str, Option<String>)> {
        vec![]
    }
}

impl TryFrom<AnnualVirtualStorageNodeV1> for AnnualVirtualStorageNode {
    type Error = ConversionError;

    fn try_from(v1: AnnualVirtualStorageNodeV1) -> Result<Self, Self::Error> {
        let meta: NodeMeta = v1.meta.into();
        let mut unnamed_count = 0;

        let cost = v1
            .cost
            .map(|v| v.try_into_v2_parameter(Some(&meta.name), &mut unnamed_count))
            .transpose()?;
        let max_volume = v1
            .max_volume
            .map(|v| v.try_into_v2_parameter(Some(&meta.name), &mut unnamed_count))
            .transpose()?;

        let min_volume = v1
            .min_volume
            .map(|v| v.try_into_v2_parameter(Some(&meta.name), &mut unnamed_count))
            .transpose()?;

        let n = Self {
            meta,
            nodes: v1.nodes,
            factors: v1.factors,
            max_volume,
            min_volume,
            cost,
            initial_volume: v1.initial_volume,
            initial_volume_pc: v1.initial_volume_pc,
            reset: AnnualReset {
                day: v1.reset_day,
                month: v1.reset_month,
                use_initial_volume: v1.reset_to_initial_volume,
            },
        };
        Ok(n)
    }
}
