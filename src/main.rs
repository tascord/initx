use std::{
    collections::BTreeMap, env::current_dir, fmt::Display, fs, path::Path, process, sync::LazyLock,
};

use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::{Input, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};

static TEMPLATES: LazyLock<Vec<Template>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    fs::read_dir(path)
        .expect("Failed to get templates directory")
        .filter_map(Result::ok)
        .filter_map(|f| {
            let path = f.path();
            let toml =
                toml::from_slice::<toml::Value>(&fs::read(path.join(".meta.toml")).ok()?).ok()?;

            let template =
                toml::from_str(&toml::to_string(toml.get("template")?).unwrap()).unwrap();

            Some(Template {
                path: path.display().to_string(),
                ..template
            })
        })
        .collect()
});

#[derive(Serialize, Deserialize)]
pub struct Template {
    name: String,
    description: String,
    alias: Vec<String>,
    commands: Vec<String>,
    ignore: Vec<String>,
    #[serde(skip)]
    path: String,
}

#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(help = "Skip name prompt for a template", short)]
    pub name: Option<String>,

    #[arg(help = "Whether to init in a dirty directory", short)]
    pub force: bool,

    #[arg(value_name = "TEMPLATE", help = "Template to install")]
    pub template: Option<String>,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    List,
}

fn bail(msg: impl Display) -> ! {
    eprintln!("✖ {}", msg.to_string().bright_red());
    process::exit(1)
}

fn apply_template(s: impl Display, vars: &BTreeMap<&str, String>) -> String {
    let contents = s.to_string();
    let mut out = String::with_capacity(contents.len());
    let mut chars = contents.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            let mut name = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_alphanumeric() || nc == '_' {
                    name.push(nc);
                    chars.next();
                } else {
                    break;
                }
            }

            if !name.is_empty() {
                if let Some(val) = vars.get(name.as_str()) {
                    out.push_str(val);
                } else {
                    // If variable is unknown, leave it untouched (put back $name)
                    out.push('$');
                    out.push_str(&name);
                }
            } else {
                // solitary '$'
                out.push('$');
            }
        } else {
            out.push(c);
        }
    }

    out
}

fn main() {
    let args = Args::parse();

    match (args.command, args.template) {
        (None, None) => bail("You need to give me something to do"),
        (Some(_), Some(_)) => bail("Can't install a template and run a command simultaneously"),

        (None, Some(t)) => {
            let t = t.to_lowercase();
            let template = TEMPLATES.iter().find(|template| {
                template.alias.iter().any(|a| *a == t) || template.name.to_lowercase() == t
            });

            if !args.force
                && fs::read_dir(current_dir().expect("Couldn't get current directory"))
                    .expect("Couldn't read current directory")
                    .next()
                    .is_some()
            {
                bail("Current directory is dirty :(")
            }

            let mut vars = BTreeMap::new();

            vars.insert(
                "location",
                current_dir().unwrap().as_path().display().to_string(),
            );

            vars.insert(
                "name",
                args.name.unwrap_or_else(|| {
                    Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Project Name")
                        .validate_with(|input: &String| -> Result<(), &str> {
                            if input.trim().is_empty() {
                                Err("Name cannot be empty")
                            } else {
                                Ok(())
                            }
                        })
                        .interact_text()
                        .unwrap()
                }),
            );

            if let Some(template) = template {
                walkdir::WalkDir::new(template.path.clone())
                    .follow_links(true)
                    .max_depth(10)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|f| !f.file_name().to_string_lossy().starts_with(".meta"))
                    .for_each(|f| {
                        let base = Path::new(&template.path);
                        let rel = f.path().strip_prefix(base).unwrap_or_else(|_| f.path());
                        let dest = current_dir().unwrap().join(rel);

                        // If it's a directory make sure it exists in destination
                        if f.file_type().is_dir() {
                            fs::create_dir_all(&dest).unwrap_or_else(|e| {
                                bail(format!(
                                    "Failed to create directory {}: {}",
                                    dest.display(),
                                    e
                                ))
                            });
                            return;
                        }

                        // Ensure parent directories exist for files
                        if let Some(parent) = dest.parent() {
                            fs::create_dir_all(parent).unwrap_or_else(|e| {
                                bail(format!(
                                    "Failed to create parent directory {}: {}",
                                    parent.display(),
                                    e
                                ))
                            });
                        }

                        // Try reading as text and perform variable replacement; if that fails, copy raw bytes
                        match fs::read_to_string(f.path()) {
                            Ok(contents) => {
                                let out = apply_template(contents, &vars);

                                fs::write(&dest, out).unwrap_or_else(|e| {
                                    bail(format!("Failed to write file {}: {}", dest.display(), e))
                                });
                            }
                            Err(_) => {
                                // Binary or unreadable as UTF-8: copy raw
                                fs::copy(f.path(), &dest).unwrap_or_else(|e| {
                                    bail(format!(
                                        "Failed to copy file to {}: {}",
                                        dest.display(),
                                        e
                                    ))
                                });
                            }
                        }
                    });

                for cmd in &template.commands {
                    let args = apply_template(cmd, &vars);
                    let mut args = args.split(" ");
                    std::process::Command::new(args.next().unwrap())
                        .args(args)
                        .current_dir(current_dir().unwrap())
                        .spawn()
                        .unwrap()
                        .wait()
                        .unwrap();
                }
            } else {
                bail(format!("No template found for {}", t))
            }
        }

        (Some(Command::List), None) => {
            println!("» {}", "Template List".bright_cyan());
            for template in TEMPLATES.iter() {
                println!(
                    "- {} {}",
                    template.name,
                    if template.alias.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "{}{}{}",
                            "(".dimmed(),
                            template
                                .alias
                                .iter()
                                .map(|a| a.to_string().italic().to_string())
                                .collect::<Vec<_>>()
                                .join(&", ".dimmed().to_string()),
                            ")".dimmed()
                        )
                    }
                )
            }
        }
    }
}
