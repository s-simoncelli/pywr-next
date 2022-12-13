use super::{NetworkState, PywrError, Recorder, RecorderMeta, Timestep};
use crate::metric::Metric;
use crate::scenario::ScenarioIndex;
use crate::state::ParameterState;
use ndarray::{s, Array2};
use std::ops::Deref;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct HDF5Recorder {
    meta: RecorderMeta,
    filename: PathBuf,
    metrics: Vec<(Metric, (String, Option<String>))>,
    file: Option<hdf5::File>,
    datasets: Option<Vec<hdf5::Dataset>>,
    aggregated_datasets: Option<Vec<(Vec<Metric>, hdf5::Dataset)>>,
    array: Option<ndarray::Array2<f64>>,
}

impl HDF5Recorder {
    pub fn new(name: &str, filename: PathBuf, metrics: Vec<(Metric, (String, Option<String>))>) -> Self {
        Self {
            meta: RecorderMeta::new(name),
            filename,
            metrics,
            file: None,
            datasets: None,
            array: None,
            aggregated_datasets: None,
        }
    }
}

impl Recorder for HDF5Recorder {
    fn meta(&self) -> &RecorderMeta {
        &self.meta
    }
    fn setup(&mut self, timesteps: &[Timestep], scenario_indices: &[ScenarioIndex]) -> Result<(), PywrError> {
        let file = match hdf5::File::create(&self.filename) {
            Ok(f) => f,
            Err(e) => return Err(PywrError::HDF5Error(e.to_string())),
        };
        let mut datasets = Vec::new();
        let mut agg_datasets = Vec::new();

        let shape = (timesteps.len(), scenario_indices.len());

        for (_metric, (name, sub_name)) in &self.metrics {
            let ds = match sub_name {
                Some(sn) => {
                    // This is a node with sub-nodes, create a group for the parent node
                    let grp = match require_group(file.deref(), name) {
                        Ok(g) => g,
                        Err(e) => return Err(PywrError::HDF5Error(e.to_string())),
                    };
                    match grp.new_dataset::<f64>().shape(shape).create(sn.as_str()) {
                        Ok(ds) => ds,
                        Err(e) => return Err(PywrError::HDF5Error(e.to_string())),
                    }
                }
                None => match file.new_dataset::<f64>().shape(shape).create(name.as_str()) {
                    Ok(ds) => ds,
                    Err(e) => return Err(PywrError::HDF5Error(e.to_string())),
                },
            };

            datasets.push(ds);
        }

        // TODO re-enable support for aggregated nodes.
        // for agg_node in model.aggregated_nodes.deref() {
        //     let metrics = agg_node.default_metric();
        //     let name = agg_node.name().to_string();
        //     println!("Adding _metric with name: {}", name);
        //     let ds = match file.new_dataset::<f64>().shape(shape).create(&*name) {
        //         Ok(ds) => ds,
        //         Err(e) => return Err(PywrError::HDF5Error(e.to_string())),
        //     };
        //     agg_datasets.push((metrics, ds));
        // }

        self.array = Some(Array2::zeros((datasets.len(), scenario_indices.len())));
        self.datasets = Some(datasets);
        self.aggregated_datasets = Some(agg_datasets);
        self.file = Some(file);

        Ok(())
    }
    fn save(
        &mut self,
        _timestep: &Timestep,
        scenario_index: &ScenarioIndex,
        network_state: &NetworkState,
        parameter_state: &ParameterState,
    ) -> Result<(), PywrError> {
        match (&mut self.array, &self.datasets) {
            (Some(array), Some(_datasets)) => {
                for (idx, (metric, _)) in self.metrics.iter().enumerate() {
                    let value = metric.get_value(network_state, parameter_state)?;
                    array[[idx, scenario_index.index]] = value
                }
                Ok(())
            }
            _ => Err(PywrError::RecorderNotInitialised),
        }?;

        match (&mut self.array, &self.aggregated_datasets) {
            (Some(array), Some(datasets)) => {
                for (idx, (metrics, _ds)) in datasets.iter().enumerate() {
                    let value: f64 = metrics
                        .iter()
                        .map(|m| m.get_value(network_state, parameter_state))
                        .sum::<Result<_, _>>()?;
                    array[[idx, scenario_index.index]] = value
                }
                Ok(())
            }
            _ => Err(PywrError::RecorderNotInitialised),
        }
    }

    fn after_save(&mut self, timestep: &Timestep) -> Result<(), PywrError> {
        match (&self.array, &mut self.datasets) {
            (Some(array), Some(datasets)) => {
                for (node_idx, dataset) in datasets.iter_mut().enumerate() {
                    if let Err(e) = dataset.write_slice(array.slice(s![node_idx, ..]), s![timestep.index, ..]) {
                        return Err(PywrError::HDF5Error(e.to_string()));
                    }
                }
                Ok(())
            }
            _ => Err(PywrError::RecorderNotInitialised),
        }?;

        match (&self.array, &mut self.aggregated_datasets) {
            (Some(array), Some(datasets)) => {
                for (node_idx, (_metric, dataset)) in datasets.iter_mut().enumerate() {
                    if let Err(e) = dataset.write_slice(array.slice(s![node_idx, ..]), s![timestep.index, ..]) {
                        return Err(PywrError::HDF5Error(e.to_string()));
                    }
                }
                Ok(())
            }
            _ => Err(PywrError::RecorderNotInitialised),
        }
    }

    fn finalise(&mut self) -> Result<(), PywrError> {
        match self.file.take() {
            Some(file) => match file.close() {
                Ok(_) => Ok(()),
                Err(e) => Err(PywrError::HDF5Error(e.to_string())),
            },
            None => Err(PywrError::RecorderNotInitialised),
        }
    }
}

fn require_group(parent: &hdf5::Group, name: &str) -> Result<hdf5::Group, hdf5::Error> {
    match parent.group(name) {
        Ok(g) => Ok(g),
        Err(_) => {
            // Group could not be retrieved already, try to create it instead
            parent.create_group(name)
        }
    }
}
