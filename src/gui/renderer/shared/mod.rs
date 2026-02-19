//! Shared layout calculations used by both CPU and GPU renderers.
//!
//! This module contains pure, side-effect-free math functions that compute
//! positions, sizes, and hit-test geometry for the tab bar.  By centralizing
//! these calculations we eliminate duplication between renderer backends.

pub mod overlay_layout;
pub mod scrollbar_math;
pub mod tab_hit_test;
pub mod tab_math;
pub mod ui_layout;
