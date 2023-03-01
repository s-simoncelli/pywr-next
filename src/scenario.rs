use crate::PywrError;

#[derive(Clone, Debug)]
pub struct ScenarioGroup {
    name: String,
    size: usize,
    // TODO labels
    // labels: Option<Vec<String>>
}

impl ScenarioGroup {
    fn new(name: &str, size: usize) -> Self {
        Self {
            name: name.to_string(),
            size,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScenarioGroupCollection {
    groups: Vec<ScenarioGroup>,
}

impl ScenarioGroupCollection {
    /// Find a `ScenarioGroup`'s index in the collection by name
    pub fn get_group_index_by_name(&self, name: &str) -> Result<usize, PywrError> {
        self.groups
            .iter()
            .position(|g| g.name == name)
            .ok_or_else(|| PywrError::ScenarioNotFound(name.to_string()))
    }

    /// Find a `ScenarioGroup`'s index in the collection by name
    pub fn get_group_by_name(&self, name: &str) -> Result<&ScenarioGroup, PywrError> {
        self.groups
            .iter()
            .find(|g| g.name == name)
            .ok_or_else(|| PywrError::ScenarioNotFound(name.to_string()))
    }

    /// Add a `ScenarioGroup` to the collection
    pub fn add_group(&mut self, name: &str, size: usize) {
        // TODO error with duplicate names
        self.groups.push(ScenarioGroup::new(name, size));
    }

    /// Return a vector of `ScenarioIndex`s for all combinations of the groups.
    pub fn scenario_indices(&self) -> Vec<ScenarioIndex> {
        let num: usize = self.groups.iter().map(|grp| grp.size).product();
        let mut scenario_indices: Vec<ScenarioIndex> = Vec::with_capacity(num);

        for scenario_id in 0..num {
            let mut remaining = scenario_id;
            let mut indices: Vec<usize> = Vec::with_capacity(self.groups.len());
            for grp in self.groups.iter().rev() {
                let idx = remaining % grp.size;
                remaining /= grp.size;
                indices.push(idx);
            }
            indices.reverse();
            scenario_indices.push(ScenarioIndex::new(scenario_id, indices));
        }
        scenario_indices
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioIndex {
    pub(crate) index: usize,
    pub(crate) indices: Vec<usize>,
}

impl ScenarioIndex {
    pub(crate) fn new(index: usize, indices: Vec<usize>) -> Self {
        Self { index, indices }
    }
}
