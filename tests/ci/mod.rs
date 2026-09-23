//! `ci.yml`: what each of its steps accepts, refuses, builds and reports,
//! one module per gate.

mod api_compatibility;
mod complexity_report;
mod duplication_report;
mod feature_combinations;
mod input_validation;
mod install_tools;
mod platform_portability;
mod quality_gates;
mod quality_reports;
mod release_payload;
mod release_payload_refusals;
mod scorecard_and_required_status;
mod scorecard_states;
mod supply_chain;
mod workspace_boundary;
