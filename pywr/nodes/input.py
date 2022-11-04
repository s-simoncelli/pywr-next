from pathlib import Path
from typing import Optional

from pywr.pywr import PyModel

from .base import BaseNode
from ..parameters import ParameterRef, BaseParameter
from ..tables import TableCollection


class InputNode(BaseNode):
    cost: Optional[ParameterRef] = None
    min_flow: Optional[ParameterRef] = None
    max_flow: Optional[ParameterRef] = None

    def create_nodes(self, r_model: PyModel):
        r_model.add_input_node(self.name)

    def set_constraints(self, r_model: PyModel, path: Path, tables: TableCollection):
        if self.cost is not None:
            cost_name = BaseParameter.ref_to_name(
                self.cost, f"{self.name}-cost", r_model, path, tables
            )
            r_model.set_node_cost(self.name, None, cost_name)
        if self.max_flow is not None:
            max_flow_name = BaseParameter.ref_to_name(
                self.max_flow, f"{self.name}-max-flow", r_model, path, tables
            )
            r_model.set_node_constraint(self.name, None, "max_flow", max_flow_name)
        if self.min_flow is not None:
            min_flow_name = BaseParameter.ref_to_name(
                self.min_flow, f"{self.name}-min-flow", r_model, path, tables
            )
            r_model.set_node_constraint(self.name, None, "min_flow", min_flow_name)
