pub mod checks;
pub mod settings;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;
    use test_case::test_case;

    use crate::linter::test_path;
    use crate::registry::CheckCode;
    use crate::{mccabe, Settings};

    #[test_case(0)]
    #[test_case(3)]
    #[test_case(10)]
    fn max_complexity_zero(max_complexity: usize) -> Result<()> {
        let snapshot = format!("max_complexity_{max_complexity}");
        let checks = test_path(
            Path::new("./resources/test/fixtures/mccabe/C901.py"),
            &Settings {
                mccabe: mccabe::settings::Settings { max_complexity },
                ..Settings::for_rules(vec![CheckCode::C901])
            },
        )?;
        insta::assert_yaml_snapshot!(snapshot, checks);
        Ok(())
    }
}
