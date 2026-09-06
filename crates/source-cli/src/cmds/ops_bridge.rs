//! Thin clap → cmd bridges for db/cache/queue/check/knowledge (keeps main thin).

use std::process::ExitCode;

use crate::cli_subs::PatternSub;
use crate::cmds::cache_cmd::{run_cache, CacheCmd};
use crate::cmds::check_cmd::{run_check, CheckCmd};
use crate::cmds::check_ops::{run_check_ops, CheckOpsCmd};
use crate::cmds::db_cmd::{run_db, DbCmd};
use crate::cmds::knowledge_cmd::{run_knowledge, KnowledgeCmd};
use crate::cmds::mcp_raw::{run_mcp_raw, McpRawCmd};
use crate::cmds::mcp_tools::{run_mcp_tools, McpToolsCmd};
use crate::cmds::pattern_cmd::{run_pattern, PatternCmd};
use crate::cmds::queue_cmd::{run_queue, QueueCmd};
use crate::cmds::queue_ops::{run_queue_ops, QueueOpsCmd};
use crate::ops_subs::{CacheSub, CheckSub, DbSub, KnowledgeSub, QueueSub};

pub fn run_db_sub(cmd: DbSub) -> ExitCode {
    match cmd {
        DbSub::Migrate => run_db(DbCmd::Migrate),
        DbSub::Status => run_db(DbCmd::Status),
        DbSub::ImportLedger { path } => run_db(DbCmd::ImportLedger { path }),
        DbSub::ImportHtmlCache { dir } => run_db(DbCmd::ImportHtmlCache { dir }),
        DbSub::ImportHostStats { path } => run_db(DbCmd::ImportHostStats { path }),
        DbSub::ImportCache => run_db(DbCmd::ImportCache),
        DbSub::ExportPhoneIndex { out } => run_db(DbCmd::ExportPhoneIndex { out }),
    }
}

pub fn run_cache_sub(cmd: CacheSub) -> ExitCode {
    match cmd {
        CacheSub::GetHtml {
            url,
            max_age,
            cache_dir,
        } => run_cache(CacheCmd::GetHtml {
            url,
            max_age,
            cache_dir,
        }),
        CacheSub::PutHtml {
            url,
            body_file,
            meta_file,
            cache_dir,
        } => run_cache(CacheCmd::PutHtml {
            url,
            body_file,
            meta_file,
            cache_dir,
        }),
        CacheSub::Cooldown {
            url,
            concurrent_rate,
            cache_dir,
        } => run_cache(CacheCmd::Cooldown {
            url,
            concurrent_rate,
            cache_dir,
        }),
        CacheSub::NoteRateLimit {
            url,
            suggested_gap,
            cache_dir,
        } => run_cache(CacheCmd::NoteRateLimit {
            url,
            suggested_gap,
            cache_dir,
        }),
        CacheSub::NoteVerify {
            url,
            success,
            duration_ms,
            used_cooldown,
            cache_dir,
        } => run_cache(CacheCmd::NoteVerify {
            url,
            success,
            duration_ms,
            used_cooldown,
            cache_dir,
        }),
        CacheSub::GetTriage {
            url,
            max_age,
            cache_dir,
        } => run_cache(CacheCmd::GetTriage {
            url,
            max_age,
            cache_dir,
        }),
        CacheSub::PutTriage {
            url,
            report_file,
            cache_dir,
        } => run_cache(CacheCmd::PutTriage {
            url,
            report_file,
            cache_dir,
        }),
    }
}

pub fn run_knowledge_sub(cmd: KnowledgeSub) -> ExitCode {
    match cmd {
        KnowledgeSub::Search { query, layer, root } => {
            run_knowledge(KnowledgeCmd::Search { query, layer, root })
        }
    }
}

pub fn run_queue_sub(cmd: QueueSub) -> ExitCode {
    match cmd {
        QueueSub::RefreshIndex { out } => run_queue(QueueCmd::RefreshIndex { out }),
        QueueSub::Rt {
            index,
            out,
            group,
            limit,
            max_rt_ms,
            full,
            all_sources,
            ledger,
        } => run_queue(QueueCmd::Rt {
            index,
            out,
            group,
            limit,
            max_rt_ms,
            full,
            all_sources,
            ledger,
        }),
        QueueSub::Cluster {
            queue,
            sources_file,
            db,
            min_size,
            out,
            from_mcp,
            limit,
        } => run_queue(QueueCmd::Cluster {
            queue,
            sources_file,
            db,
            min_size,
            out,
            from_mcp,
            limit,
        }),
        QueueSub::Build { input, out, limit } => {
            run_queue_ops(QueueOpsCmd::Build { input, out, limit })
        }
        QueueSub::Classify {
            fail_msg,
            url,
            html,
            html_file,
        } => run_queue_ops(QueueOpsCmd::Classify {
            fail_msg,
            url,
            html,
            html_file,
        }),
        QueueSub::Why { input, out } => run_queue_ops(QueueOpsCmd::Why { input, out }),
    }
}

pub fn run_check_sub(cmd: CheckSub) -> ExitCode {
    match cmd {
        CheckSub::Channel {
            clear_stale,
            force_clear,
            reset_remote,
        } => run_check(CheckCmd::Channel {
            clear_stale,
            force_clear,
            reset_remote,
        }),
        CheckSub::Precheck {
            urls_file,
            timeout,
            concurrency,
            out,
        } => run_check(CheckCmd::Precheck {
            urls_file,
            timeout,
            concurrency,
            out,
        }),
        CheckSub::Batch {
            urls_file,
            keyword,
            batch_size,
            thread_count,
            timeout,
            materials_dir,
            report,
        } => run_check(CheckCmd::Batch {
            urls_file,
            keyword,
            batch_size,
            thread_count,
            timeout,
            materials_dir,
            report_path: report,
        }),
        CheckSub::Full {
            urls_file,
            keyword,
            batch_size,
            thread_count,
            timeout,
            precheck_json,
            materials_dir,
            report,
        } => run_check(CheckCmd::Full {
            urls_file,
            keyword,
            batch_size,
            thread_count,
            timeout,
            precheck_json,
            materials_dir,
            report_path: report,
        }),
        CheckSub::Shard {
            urls_file,
            nodes,
            virtual_nodes,
            out,
        } => run_check_ops(CheckOpsCmd::Shard {
            urls_file,
            nodes,
            virtual_nodes,
            out,
        }),
        CheckSub::DisableDead {
            precheck_json,
            disable,
            tag,
            limit,
            out,
            dry_run,
        } => run_check_ops(CheckOpsCmd::DisableDead {
            precheck_json,
            disable,
            tag,
            limit,
            out,
            dry_run,
        }),
        CheckSub::Prefilter {
            urls_file,
            out,
            concurrency,
            l2_timeout,
            rules,
        } => run_check_ops(CheckOpsCmd::Prefilter {
            urls_file,
            out,
            concurrency,
            l2_timeout,
            rules,
        }),
        CheckSub::ClearCookies { url } => run_mcp_tools(McpToolsCmd::ClearCookies { url }),
        CheckSub::CookieGet { url } => run_mcp_raw(McpRawCmd::CookieGet { url }),
        CheckSub::CookieSet { url, cookie } => run_mcp_raw(McpRawCmd::CookieSet { url, cookie }),
        CheckSub::LogRecording { enabled } => run_mcp_raw(McpRawCmd::LogRecording { enabled }),
        CheckSub::LogList { limit } => run_mcp_raw(McpRawCmd::LogList { limit }),
        CheckSub::LogGet { id } => run_mcp_raw(McpRawCmd::LogGet { id }),
    }
}

pub fn run_pattern_sub(cmd: PatternSub) -> ExitCode {
    match cmd {
        PatternSub::Extract {
            sources_file,
            db,
            out_dir,
            min_size,
            limit,
            write_db,
            from_mcp,
            enabled_only,
            fixed_only,
        } => run_pattern(PatternCmd::Extract {
            sources_file,
            db,
            out_dir,
            min_size,
            limit,
            write_db,
            from_mcp,
            enabled_only,
            fixed_only,
        }),
    }
}
