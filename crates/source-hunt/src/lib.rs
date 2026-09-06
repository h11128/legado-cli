//! Domain hunt from curated seeds (`config/domain_hunt_seeds.json`).

mod decide;
mod seeds;

pub use decide::{decide_hunt_action, HuntAction, HuntResolve};
pub use seeds::{
    hunt_candidates, hunt_candidates_by_keyword, seed_candidates, HuntSeeds, SeedEntry,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HuntError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn zxcs_seed_lookup() {
        let seeds: HuntSeeds = serde_json::from_str(
            r#"{"seeds":{"www.zxcs.info":{"migrated_to":"https://www.zxcs.click/","candidates":["https://www.zxcs.click/","https://www.zxcs.live/"]}}}"#,
        )
        .unwrap();
        let c = hunt_candidates(&seeds, "http://www.zxcs.info/");
        assert_eq!(c[0], "https://www.zxcs.click/");
        assert!(c.len() >= 2);
    }

    #[test]
    fn temp_seeds_json_and_keyword() {
        let dir =
            std::env::temp_dir().join(format!("source_hunt_seeds_{}.json", std::process::id()));
        {
            let mut f = std::fs::File::create(&dir).unwrap();
            write!(
                f,
                r#"{{"seeds":{{
                  "www.zxcs.info":{{"note":"zxcs family","migrated_to":"https://www.zxcs.click/","candidates":["https://www.zxcs.live/"]}},
                  "book.tiexue.net":{{"note":"shutdown","shutdown":true,"candidates":["https://book.tiexue.net/"]}}
                }}}}"#
            )
            .unwrap();
        }
        let got = seed_candidates(Some(&dir), "http://www.zxcs.info/", None).unwrap();
        assert!(got.iter().any(|u| u.contains("zxcs.click")));
        let by_kw = seed_candidates(Some(&dir), "http://ignored/", Some("zxcs")).unwrap();
        assert!(by_kw.iter().any(|u| u.contains("zxcs")));
        assert!(!by_kw.iter().any(|u| u.contains("tiexue")));
        let _ = std::fs::remove_file(&dir);
    }
}
