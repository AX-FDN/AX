use super::*;

#[derive(Debug, PartialEq, Eq)]
pub(in crate::cli) struct CheckOptions {
    pub(in crate::cli) file: PathBuf,
    pub(in crate::cli) json: bool,
    pub(in crate::cli) ai: bool,
    pub(in crate::cli) ai_session: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::cli) struct BuildCliOptions {
    pub(in crate::cli) file: PathBuf,
    pub(in crate::cli) out_dir: Option<PathBuf>,
    pub(in crate::cli) json: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::cli) struct LockCliOptions {
    pub(in crate::cli) file: PathBuf,
    pub(in crate::cli) check: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::cli) struct RunOptions {
    pub(in crate::cli) file: PathBuf,
    pub(in crate::cli) json: bool,
    pub(in crate::cli) ai: bool,
    pub(in crate::cli) ai_session: Option<PathBuf>,
    pub(in crate::cli) argv: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::cli) struct ContextOptions {
    pub(in crate::cli) view: ContextView,
    pub(in crate::cli) file: PathBuf,
    pub(in crate::cli) symbol: Option<String>,
}

pub(in crate::cli) fn parse_check_args(args: Vec<String>) -> Result<CheckOptions, String> {
    let mut json = false;
    let mut ai = false;
    let mut ai_session = None;
    let mut file = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--json" => {
                json = true;
            }
            "--ai" => {
                ai = true;
            }
            "--ai-session" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("missing path after `--ai-session`".to_string());
                };
                ai_session = Some(PathBuf::from(path));
                index += 1;
            }
            _ if arg.starts_with("--ai-session=") => {
                let path = arg
                    .split_once('=')
                    .map(|(_, value)| value)
                    .unwrap_or_default();
                if path.is_empty() {
                    return Err("missing path after `--ai-session=`".to_string());
                }
                ai_session = Some(PathBuf::from(path));
            }
            _ if file.is_none() => {
                file = Some(PathBuf::from(arg));
            }
            _ => {
                return Err(format!("unexpected argument `{arg}`"));
            }
        }
        index += 1;
    }

    let Some(file) = file else {
        return Err("missing input file for `axc check`".to_string());
    };

    if ai && !json {
        return Err("`--ai` requires `--json`".to_string());
    }

    if ai_session.is_some() && !ai {
        return Err("`--ai-session` requires `--ai`".to_string());
    }

    Ok(CheckOptions {
        file,
        json,
        ai,
        ai_session,
    })
}

pub(in crate::cli) fn parse_run_args(args: Vec<String>) -> Result<RunOptions, String> {
    let mut json = false;
    let mut ai = false;
    let mut ai_session = None;
    let mut argv = Vec::new();
    let mut file = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--" => {
                argv.extend(args[index + 1..].iter().cloned());
                break;
            }
            "--json" => {
                json = true;
            }
            "--ai" => {
                ai = true;
            }
            "--ai-session" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("missing path after `--ai-session`".to_string());
                };
                ai_session = Some(PathBuf::from(path));
                index += 1;
            }
            _ if arg.starts_with("--ai-session=") => {
                let path = arg
                    .split_once('=')
                    .map(|(_, value)| value)
                    .unwrap_or_default();
                if path.is_empty() {
                    return Err("missing path after `--ai-session=`".to_string());
                }
                ai_session = Some(PathBuf::from(path));
            }
            _ if file.is_none() => {
                file = Some(PathBuf::from(arg));
            }
            _ => {
                return Err(format!("unexpected argument `{arg}`"));
            }
        }
        index += 1;
    }

    let Some(file) = file else {
        return Err("missing input file for `axc run`".to_string());
    };

    if ai && !json {
        return Err("`--ai` requires `--json`".to_string());
    }

    if ai_session.is_some() && !ai {
        return Err("`--ai-session` requires `--ai`".to_string());
    }

    Ok(RunOptions {
        file,
        json,
        ai,
        ai_session,
        argv,
    })
}

pub(in crate::cli) fn parse_build_args(args: Vec<String>) -> Result<BuildCliOptions, String> {
    let mut json = false;
    let mut out_dir = None;
    let mut file = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--json" => {
                json = true;
            }
            "--out-dir" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("missing path after `--out-dir`".to_string());
                };
                out_dir = Some(PathBuf::from(path));
                index += 1;
            }
            _ if arg.starts_with("--out-dir=") => {
                let path = arg
                    .split_once('=')
                    .map(|(_, value)| value)
                    .unwrap_or_default();
                if path.is_empty() {
                    return Err("missing path after `--out-dir=`".to_string());
                }
                out_dir = Some(PathBuf::from(path));
            }
            _ if file.is_none() => {
                file = Some(PathBuf::from(arg));
            }
            _ => {
                return Err(format!("unexpected argument `{arg}`"));
            }
        }
        index += 1;
    }

    let Some(file) = file else {
        return Err("missing input file for `axc build`".to_string());
    };

    Ok(BuildCliOptions {
        file,
        out_dir,
        json,
    })
}

pub(in crate::cli) fn parse_lock_args(args: Vec<String>) -> Result<LockCliOptions, String> {
    let mut file = None;
    let mut check = false;

    for arg in args {
        match arg.as_str() {
            "--check" => check = true,
            _ if file.is_none() => {
                file = Some(PathBuf::from(arg));
            }
            _ => {
                return Err(format!("unexpected argument `{arg}`"));
            }
        }
    }

    let Some(file) = file else {
        return Err("missing input project for `axc lock`".to_string());
    };

    Ok(LockCliOptions { file, check })
}

pub(in crate::cli) fn parse_context_args(args: Vec<String>) -> Result<ContextOptions, String> {
    let Some((view, rest)) = args.split_first() else {
        return Err("missing context view for `axc context`".to_string());
    };

    let view = match view.as_str() {
        "overview" => ContextView::Overview,
        "boundaries" => ContextView::Boundaries,
        "topology" => ContextView::Topology,
        "flow" => ContextView::Flow,
        "symbol" => ContextView::Symbol,
        "impact" => ContextView::Impact,
        "evidence" => ContextView::Evidence,
        _ => {
            return Err(format!(
                "unknown context view `{view}`; expected `overview`, `boundaries`, `topology`, `flow`, `symbol`, `impact`, or `evidence`"
            ));
        }
    };

    let mut file = None;
    let mut symbol = None;
    for arg in rest {
        match arg.as_str() {
            "--json" => {}
            _ if file.is_none() => {
                file = Some(PathBuf::from(arg));
            }
            _ if matches!(
                view,
                ContextView::Symbol | ContextView::Impact | ContextView::Evidence
            ) && symbol.is_none() =>
            {
                symbol = Some(arg.clone());
            }
            _ => return Err(format!("unexpected argument `{arg}`")),
        }
    }

    let Some(file) = file else {
        return Err(format!(
            "missing input path for `axc context {}`",
            view.as_str()
        ));
    };

    if matches!(
        view,
        ContextView::Symbol | ContextView::Impact | ContextView::Evidence
    ) && symbol.is_none()
    {
        return Err(format!(
            "missing symbol query for `axc context {}`",
            view.as_str()
        ));
    }

    Ok(ContextOptions { view, file, symbol })
}

pub(in crate::cli) fn load_input(path: &Path) -> Result<ResolvedInput, String> {
    resolve_input(path).map_err(|error| append_package_repair_hint(&error))
}
