use std::{
    collections::HashSet,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{TriggerHooks, SPIN_HOME};

/// Which components should have their logs followed on stdout/stderr.
#[derive(Clone, Debug)]
pub enum FollowComponents {
    /// No components should have their logs followed.
    None,
    /// Only the specified components should have their logs followed.
    Named(HashSet<String>),
    /// All components should have their logs followed.
    All,
}

impl FollowComponents {
    /// Whether a given component should have its logs followed on stdout/stderr.
    pub fn should_follow(&self, component_id: &str) -> bool {
        match self {
            Self::None => false,
            Self::All => true,
            Self::Named(ids) => ids.contains(component_id),
        }
    }
}

impl Default for FollowComponents {
    fn default() -> Self {
        Self::None
    }
}

/// Implements TriggerHooks, writing logs to a log file and (optionally) stderr
pub struct StdioLoggingTriggerHooks {
    follow_components: FollowComponents,
    log_dir: Option<PathBuf>,
}

impl StdioLoggingTriggerHooks {
    pub fn new(follow_components: FollowComponents, log_dir: Option<PathBuf>) -> Self {
        Self {
            follow_components,
            log_dir,
        }
    }

    fn component_stdio_writer(
        &self,
        component_id: &str,
        log_suffix: &str,
    ) -> Result<ComponentStdioWriter> {
        let sanitized_component_id = sanitize_filename::sanitize(component_id);
        let follow = self.follow_components.should_follow(component_id);

        if let Some(log_dir) = self.log_dir {
            let log_path = log_dir
                .as_deref()
                .expect("log_dir should have been initialized in app_loaded")
                .join(format!("{sanitized_component_id}_{log_suffix}.txt"));
            ComponentStdioWriter::new(Some(&log_path), follow)
                .with_context(|| format!("Failed to open log file {log_path:?}"))
        } else {
            ComponentStdioWriter::new(None, follow)
        }
    }

    fn validate_follows(&self, app: &spin_app::App) -> anyhow::Result<()> {
        match &self.follow_components {
            FollowComponents::Named(names) => {
                let component_ids: HashSet<_> =
                    app.components().map(|c| c.id().to_owned()).collect();
                let unknown_names: Vec<_> = names.difference(&component_ids).collect();
                if unknown_names.is_empty() {
                    Ok(())
                } else {
                    let unknown_list = bullet_list(&unknown_names);
                    let actual_list = bullet_list(&component_ids);
                    let message = anyhow::anyhow!("The following component(s) specified in --follow do not exist in the application:\n{unknown_list}\nThe following components exist:\n{actual_list}");
                    Err(message)
                }
            }
            _ => Ok(()),
        }
    }
}

impl TriggerHooks for StdioLoggingTriggerHooks {
    fn app_loaded(&mut self, app: &spin_app::App) -> anyhow::Result<()> {
        let app_name: &str = app.require_metadata("name")?;

        self.validate_follows(app)?;

        // Ensure log_dir exists if passed
        if let Some(log_dir) = self.log_dir {
            log_dir.get_or_insert_with(|| {
                let parent_dir = match dirs::home_dir() {
                    Some(home) => home.join(SPIN_HOME),
                    None => PathBuf::new(), // "./"
                };
                let sanitized_app = sanitize_filename::sanitize(app_name);
                parent_dir.join(sanitized_app).join("logs")
            });

            std::fs::create_dir_all(&log_dir)
                .with_context(|| format!("Failed to create log dir {log_dir:?}"))?;
        }

        Ok(())
    }

    fn component_store_builder(
        &self,
        component: spin_app::AppComponent,
        builder: &mut spin_core::StoreBuilder,
        log_file: Option<PathBuf>,
    ) -> anyhow::Result<()> {
        builder.stdout_pipe(self.component_stdio_writer(component.id(), "stdout")?);
        builder.stderr_pipe(self.component_stdio_writer(component.id(), "stderr")?);

        // builder.inherit_stdout();
        // builder.inherit_stderr();

        Ok(())
    }
}

/// ComponentStdioWriter specifies where to forward output
///     Output can be forwarded to a log file and/or stdout
pub struct ComponentStdioWriter {
    log_file: Option<File>,
    follow: bool,
}

impl ComponentStdioWriter {
    pub fn new(log_path: Option<&Path>, follow: bool) -> anyhow::Result<Self> {
        match log_path {
            Some(p) => {
                let log_file = File::options().create(true).append(true).open(p)?;
                Ok(Self {
                    log_file: Some(log_file),
                    follow,
                })
            }
            None => Ok(Self {
                log_file: None,
                follow,
            }),
        }
    }
}

impl std::io::Write for ComponentStdioWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut written: usize;
        if self.log_file {
            written = self.log_file.write(buf)?;

            if self.follow {
                std::io::stderr().write_all(&buf[..written])?;
            }
        }
        if self.follow {
            written = std::io::stderr().write_all(buf)?;
        }
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.log_file.flush()?;
        if self.follow {
            std::io::stderr().flush()?;
        }
        Ok(())
    }
}

fn bullet_list<S: std::fmt::Display>(items: impl IntoIterator<Item = S>) -> String {
    items
        .into_iter()
        .map(|item| format!("  - {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}
