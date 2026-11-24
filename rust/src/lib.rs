//! Gaden: closed-loop appetite-circuit simulation. Research only. Not a medical device.
//!
//! - [`neuron`], [`pomc`], [`control`]: v0.1 port of the Python reference (parity-tested).
//! - [`hormones`]: ghrelin, leptin, insulin, GLP-1 and glucose with circadian timing.
//! - [`controllers`]: PID, MPC and a bandit-tuned PI.
//! - [`sim`]: 24-h closed loop and the 240-scenario stress suite.
//! - [`fit`]: fit to published summary values in `data/targets.csv`.

pub mod anchors;
pub mod control;
pub mod controllers;
pub mod fit;
pub mod hormones;
pub mod neuron;
pub mod pomc;
pub mod sim;
