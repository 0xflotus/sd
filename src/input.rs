use crate::{utils, Result};
use regex::Regex;
use std::{fs::File, io::prelude::*};

pub(crate) enum Source {
    Stdin,
    Files(Vec<String>),
}

impl Source {
    pub(crate) fn from(file_paths: Vec<String>) -> Self {
        if file_paths.len() == 0 {
            return Source::Stdin;
        }
        return Source::Files(file_paths);
    }

    fn file_to_string(path: impl AsRef<str>) -> Result<String> {
        let mut file = File::open(path.as_ref())?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;
        Ok(buffer)
    }
}

pub(crate) enum Replacer<'a> {
    Regex(Regex, &'a str),
    Literal(&'a str, &'a str),
}

impl<'a> Replacer<'a> {
    pub(crate) fn new(
        look_for: &'a str,
        replace_with: &'a str,
        is_literal: bool,
    ) -> Result<Self> {
        if is_literal {
            return Ok(Replacer::Literal(look_for, replace_with));
        }
        return Ok(Replacer::Regex(regex::Regex::new(look_for)?, replace_with));
    }

    pub(crate) fn replace(&self, content: &str) -> String {
        match self {
            Replacer::Regex(regex, replace_with) => {
                let replaced =
                    regex.replace_all(&content, *replace_with).to_string();
                utils::unescape(&replaced).unwrap_or_else(|| replaced)
            },
            Replacer::Literal(search, replace_with) => {
                content.replace(search, replace_with)
            },
        }
    }

    pub(crate) fn run(&self, source: &Source, in_place: bool) -> Result<()> {
        use atomic_write::atomic_write;
        use rayon::prelude::*;

        match source {
            Source::Stdin => {
                let mut buffer = String::new();
                let stdin = std::io::stdin();
                let mut handle = stdin.lock();
                handle.read_to_string(&mut buffer)?;

                let stdout = std::io::stdout();
                let mut handle = stdout.lock();
                handle.write(&self.replace(&buffer).as_bytes())?;
                Ok(())
            },
            Source::Files(paths) => {
                if in_place {
                    paths
                        .par_iter()
                        .map(|p| {
                            Ok(atomic_write(
                                p,
                                self.replace(&Source::file_to_string(p)?),
                            )?)
                        })
                        .collect::<Result<Vec<()>>>()?;
                    Ok(())
                }
                else {
                    let stdout = std::io::stdout();
                    let mut handle = stdout.lock();

                    paths
                        .iter()
                        .map(|p| {
                            handle.write(
                                &self
                                    .replace(&Source::file_to_string(p)?)
                                    .as_bytes(),
                            )?;
                            Ok(())
                        })
                        .collect::<Result<Vec<()>>>()?;
                    Ok(())
                }
            },
        }
    }
}
