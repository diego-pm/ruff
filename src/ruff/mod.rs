//! Module for Ruff-specific rules.

pub mod checks;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;
    use rustc_hash::FxHashSet;
    use test_case::test_case;

    use crate::linter::test_path;
    use crate::registry::CheckCode;
    use crate::settings;
    #[test_case(CheckCode::RUF004, Path::new("RUF004.py"); "RUF004")]
    fn checks(check_code: CheckCode, path: &Path) -> Result<()> {
        let snapshot = format!("{}_{}", check_code.as_ref(), path.to_string_lossy());
        let checks = test_path(
            Path::new("./resources/test/fixtures/ruff")
                .join(path)
                .as_path(),
            &settings::Settings::for_rule(check_code),
        )?;
        insta::assert_yaml_snapshot!(snapshot, checks);
        Ok(())
    }

    #[test]
    fn confusables() -> Result<()> {
        let checks = test_path(
            Path::new("./resources/test/fixtures/ruff/confusables.py"),
            &settings::Settings {
                allowed_confusables: FxHashSet::from_iter(['−', 'ρ', '∗']),
                ..settings::Settings::for_rules(vec![
                    CheckCode::RUF001,
                    CheckCode::RUF002,
                    CheckCode::RUF003,
                ])
            },
        )?;
        insta::assert_yaml_snapshot!(checks);
        Ok(())
    }

    #[test]
    fn ruf100_0() -> Result<()> {
        let checks = test_path(
            Path::new("./resources/test/fixtures/ruff/RUF100_0.py"),
            &settings::Settings::for_rules(vec![
                CheckCode::RUF100,
                CheckCode::E501,
                CheckCode::F401,
                CheckCode::F841,
            ]),
        )?;
        insta::assert_yaml_snapshot!(checks);
        Ok(())
    }

    #[test]
    fn ruf100_1() -> Result<()> {
        let checks = test_path(
            Path::new("./resources/test/fixtures/ruff/RUF100_1.py"),
            &settings::Settings::for_rules(vec![CheckCode::RUF100, CheckCode::F401]),
        )?;
        insta::assert_yaml_snapshot!(checks);
        Ok(())
    }

    #[test]
    fn flake8_noqa() -> Result<()> {
        let checks = test_path(
            Path::new("./resources/test/fixtures/ruff/flake8_noqa.py"),
            &settings::Settings::for_rules(vec![CheckCode::F401, CheckCode::F841]),
        )?;
        insta::assert_yaml_snapshot!(checks);
        Ok(())
    }

    #[test]
    fn ruff_noqa() -> Result<()> {
        let checks = test_path(
            Path::new("./resources/test/fixtures/ruff/ruff_noqa.py"),
            &settings::Settings::for_rules(vec![CheckCode::F401, CheckCode::F841]),
        )?;
        insta::assert_yaml_snapshot!(checks);
        Ok(())
    }

    #[test]
    fn redirects() -> Result<()> {
        let checks = test_path(
            Path::new("./resources/test/fixtures/ruff/redirects.py"),
            &settings::Settings::for_rules(vec![CheckCode::UP007]),
        )?;
        insta::assert_yaml_snapshot!(checks);
        Ok(())
    }
}
