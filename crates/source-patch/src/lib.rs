//! Apply dotted-path PatchOps and safe smell/auto patches onto BookSource JSON.

mod apply;
mod auto;
mod scheme;
mod smell;
mod smells;

pub use apply::{apply_ops, ApplyError};
pub use auto::apply_auto_patches;
pub use scheme::normalize_source_schemes;
pub use smell::{is_rate_only_ops, strip_dead_explore_hint};
pub use smells::{
    apply_safe_rule_fixes, fix_17mb_empty_index_toc, fix_bookurl_class_space, fix_webview_quotes,
    is_17mb_empty_index_toc, smell_rules, TOC_17MB_STATIC,
};
