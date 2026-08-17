use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Cursor, IsTerminal, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Archive;

const NODE_2_HELP: &str = include_str!("../fixtures/node-oracle/help.txt");
const SNAPSHOT_ARCHIVE: &[u8] = include_bytes!("../assets/template.snapshot.tar");
const SNAPSHOT_COMMIT: &str = "68c367a13d5006cca83f1c5e369678af28c4bf15";
const SNAPSHOT_HASH: &str = "f4276bfa8e6ca7781f905372d912f8fd9ba806566e212550b4548eda0f877387";
const SNAPSHOT_ARCHIVE_SHA256: &str =
    "f72c6bd76c48247ec31245f150be257b9eeb4388da32a29a5e958d3b2600778e";
const TEMPLATE_MANIFEST_SHA256: &str =
    "48549af09ac85a9e0caf97d9342e8ee31b1cc8b608704bc9f1aa0d546f9a635c";
const SNAPSHOT_BINDING_ROOT: &str = "__yss_runtime";
const TEMPLATE_SOURCE: &str = "github:iloveZzz/yss-spec-project-template";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

const RENDER_PATHS: &[&str] = &["AGENTS.md", "README.md", "yss-project.yaml"];
const EXAMPLE_DOC_PATH: &str = "docs/discovery/IDEATION.md";
const EXCLUDED_PATHS: &[&str] = &[
    ".claude/settings.local.json",
    ".codex/hooks.json",
    ".codex/settings.local.json",
    ".codex/skills/.DS_Store",
    ".pi/settings.json",
];
const EXCLUDED_ROOT_ENTRIES: &[&str] = &[
    ".git",
    ".codegraph",
    ".codebuddy",
    ".idea",
    ".uploads",
    "node_modules",
    ".ua",
    ".qwen",
    ".template-source",
    "packages",
];
const EXCLUDED_ROOT_FILES: &[&str] = &[
    "package-lock.json",
    "package.json",
    "template.manifest.json",
    "template.snapshot.json",
];
const AGENT_SKILL_ROOTS: &[&str] = &[
    ".agents/skills",
    ".claude/skills",
    ".codex/skills",
    ".hermes/skills",
    ".pi/skills",
    ".qoder/skills",
    ".trae/skills",
];

#[derive(Debug, Clone, Default)]
struct Options {
    project_name: Option<String>,
    business_domain: Option<String>,
    team_size: Option<String>,
    target_dir: Option<PathBuf>,
    issue_tracker: Option<String>,
    dry_run: bool,
    apply: bool,
    force: bool,
    git_init: bool,
    include_example_docs: Option<bool>,
    help: bool,
}

#[derive(Debug, Clone)]
struct Variables {
    project_name: String,
    business_domain: String,
    team_size: String,
    issue_tracker: String,
    include_example_docs: bool,
}

#[derive(Debug, Clone)]
struct ManagedFile {
    path: String,
    kind: &'static str,
    content_hash: String,
}

#[derive(Debug, Clone)]
struct DesiredFile {
    path: String,
    kind: &'static str,
    content: Vec<u8>,
    mode: u32,
}

#[derive(Debug, Default)]
struct MigrationPlan {
    moves: Vec<(PathBuf, PathBuf)>,
    removes: Vec<PathBuf>,
    conflicts: Vec<String>,
    unsafe_paths: Vec<String>,
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    if args
        .first()
        .is_some_and(|argument| argument == "--help" || argument == "-h")
    {
        print_help();
        return Ok(());
    }
    if args.first().is_some_and(|argument| argument == "attach") {
        return run_attach(&args[1..]);
    }
    if args.first().is_some_and(|argument| argument == "sync") {
        return run_sync(&args[1..]);
    }
    if args
        .first()
        .is_some_and(|argument| argument == "verify-template")
    {
        return run_verify_template(&args[1..]);
    }

    let options = parse_args(&args)?;
    if options.help {
        print_help();
        return Ok(());
    }
    run_init(options)
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut index = 0;
    while index < args.len() {
        let current = &args[index];
        let value = |name: &str, index: &mut usize| -> Result<String, String> {
            let next = args
                .get(*index + 1)
                .ok_or_else(|| format!("{name} 需要一个值"))?;
            if next.starts_with("--") {
                return Err(format!("{name} 需要一个值"));
            }
            *index += 1;
            Ok(next.clone())
        };
        match current.as_str() {
            "--project-name" => options.project_name = Some(value(current, &mut index)?),
            "--business-domain" => options.business_domain = Some(value(current, &mut index)?),
            "--team-size" => options.team_size = Some(value(current, &mut index)?),
            "--target-dir" => options.target_dir = Some(PathBuf::from(value(current, &mut index)?)),
            "--issue-tracker" => options.issue_tracker = Some(value(current, &mut index)?),
            "--dry-run" => options.dry_run = true,
            "--apply" => options.apply = true,
            "--force" => options.force = true,
            "--git-init" => options.git_init = true,
            "--include-example-docs" => options.include_example_docs = Some(true),
            "--no-example-docs" => options.include_example_docs = Some(false),
            "--help" | "-h" => options.help = true,
            other => return Err(format!("不支持的参数：{other}")),
        }
        index += 1;
    }
    Ok(options)
}

fn variables(options: &Options) -> Result<Variables, String> {
    variables_for(options, "init")
}

fn variables_for(options: &Options, command: &str) -> Result<Variables, String> {
    let project_name = options
        .project_name
        .clone()
        .ok_or_else(|| format!("{command} 需要 --project-name，项目名称不能为空"))?;
    let business_domain = options
        .business_domain
        .clone()
        .ok_or_else(|| format!("{command} 需要 --business-domain，业务领域不能为空"))?;
    let target_dir = options
        .target_dir
        .as_ref()
        .ok_or_else(|| format!("{command} 需要 --target-dir，目标目录不能为空"))?;
    if project_name.trim().is_empty() {
        return Err(format!("{command} 需要 --project-name，项目名称不能为空"));
    }
    if business_domain.trim().is_empty() {
        return Err(format!(
            "{command} 需要 --business-domain，业务领域不能为空"
        ));
    }
    if target_dir.as_os_str().is_empty() {
        return Err(format!("{command} 需要 --target-dir，目标目录不能为空"));
    }
    Ok(Variables {
        project_name,
        business_domain,
        team_size: options
            .team_size
            .clone()
            .unwrap_or_else(|| "待补充".to_owned()),
        issue_tracker: options
            .issue_tracker
            .clone()
            .unwrap_or_else(|| "github".to_owned()),
        include_example_docs: options.include_example_docs.unwrap_or(true),
    })
}

fn run_init(options: Options) -> Result<(), String> {
    let options = resolve_init_options(options)?;
    let variables = variables(&options)?;
    let target_dir = absolute_path(options.target_dir.as_ref().expect("validated target"))?;
    let snapshot = Snapshot::load()?;
    let files = snapshot.files()?;

    if options.dry_run {
        println!("dry-run 预览");
        println!("输出目录：{}", target_dir.display());
        println!("模板快照：{}", SNAPSHOT_COMMIT);
        println!("snapshotHash：{}", SNAPSHOT_HASH);
        for path in files {
            let kind = if RENDER_PATHS.contains(&path.as_str()) {
                "render"
            } else {
                "copy"
            };
            if !variables.include_example_docs && path == EXAMPLE_DOC_PATH {
                continue;
            }
            println!("{kind}: {path}");
        }
        return Ok(());
    }

    let (backup, preexisting_empty_target) = prepare_target(&target_dir, options.force)?;
    if let Err(error) = initialize_target(&target_dir, &variables, &snapshot, options.git_init) {
        if let Some(backup_path) = backup.as_ref() {
            restore_backup(&target_dir, backup_path)?;
        } else if preexisting_empty_target {
            clear_directory_contents(&target_dir)?;
        } else if target_dir.exists() {
            let _ = fs::remove_dir_all(&target_dir);
        }
        return Err(format!("{error}\n已回滚本次 init"));
    }

    println!("初始化完成");
    println!("输出目录：{}", target_dir.display());
    if let Some(backup_path) = backup {
        println!("备份目录：{}", backup_path.display());
    }
    println!("下一步建议：");
    println!("1. cd {}", target_dir.display());
    println!("2. 如需版本管理，可执行 git init");
    println!("3. 检查 AGENTS.md、README 和 docs 目录是否符合预期");
    Ok(())
}

fn resolve_init_options(mut options: Options) -> Result<Options, String> {
    if options.project_name.is_some()
        && options.business_domain.is_some()
        && options.target_dir.is_some()
    {
        return Ok(options);
    }
    if io::stdin().is_terminal() {
        let ask = |label: &str| -> Result<String, String> {
            print!("{label}: ");
            io::stdout()
                .flush()
                .map_err(|error| format!("无法刷新交互提示：{error}"))?;
            let mut answer = String::new();
            io::stdin()
                .read_line(&mut answer)
                .map_err(|error| format!("无法读取交互输入：{error}"))?;
            Ok(answer.trim().to_owned())
        };
        if options.project_name.is_none() {
            options.project_name = Some(ask("项目名称")?);
        }
        if options.business_domain.is_none() {
            options.business_domain = Some(ask("业务领域")?);
        }
        if options.team_size.is_none() {
            let answer = ask("团队规模（可留空）")?;
            options.team_size = Some(if answer.is_empty() {
                "待补充".to_owned()
            } else {
                answer
            });
        }
        if options.target_dir.is_none() {
            options.target_dir = Some(PathBuf::from(ask("目标目录")?));
        }
    } else {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("无法读取交互输入：{error}"))?;
        let mut answers = input.lines();
        let mut ask = |label: &str| -> String {
            print!("{label}: ");
            let _ = io::stdout().flush();
            answers.next().unwrap_or_default().trim().to_owned()
        };
        if options.project_name.is_none() {
            options.project_name = Some(ask("项目名称"));
        }
        if options.business_domain.is_none() {
            options.business_domain = Some(ask("业务领域"));
        }
        if options.team_size.is_none() {
            let answer = ask("团队规模（可留空）");
            options.team_size = Some(if answer.is_empty() {
                "待补充".to_owned()
            } else {
                answer
            });
        }
        if options.target_dir.is_none() {
            options.target_dir = Some(PathBuf::from(ask("目标目录")));
        }
    }
    Ok(options)
}

fn run_attach(args: &[String]) -> Result<(), String> {
    let options = parse_args(args)?;
    if options.help {
        print_help();
        return Ok(());
    }
    let variables = variables_for(&options, "attach")?;
    if options.dry_run && options.apply {
        return Err("attach 的 --dry-run 与 --apply 互斥".to_owned());
    }
    if !options.dry_run && !options.apply {
        return Err("attach 必须显式传入 --dry-run 或 --apply".to_owned());
    }
    let target = absolute_path(options.target_dir.as_ref().expect("validated target"))?;
    ensure_existing_directory(&target, "attach 目标目录必须是已经存在的项目目录")?;
    if target.join(".yss-template.json").exists() {
        return Err("当前项目已有模板元数据，请使用 sync，不要重复 attach".to_owned());
    }
    let snapshot = Snapshot::load()?;
    let desired = desired_for_target(&snapshot, &variables, &target)?;
    let plan = classify_attach(&target, &desired)?;
    let migration = build_migration_plan(&target, &desired)?;
    if options.dry_run {
        print_attach_dry_run(&target, &plan, &migration);
        return Ok(());
    }
    if !migration.unsafe_paths.is_empty() {
        return Err(format!(
            "attach 被 unsafe 迁移项阻断：{}",
            migration.unsafe_paths.join(", ")
        ));
    }
    if !migration.conflicts.is_empty() {
        return Err(format!(
            "attach 被旧路径迁移冲突阻断：{}",
            migration.conflicts.join("；")
        ));
    }
    if !plan.conflicts.is_empty() && !options.force {
        print_attach_dry_run(&target, &plan, &migration);
        return Err("attach 检测到受管文件冲突；请先 dry-run，再使用 --apply --force".to_owned());
    }
    let mut transaction = Transaction::begin(&target)?;
    let apply_paths: BTreeMap<String, DesiredFile> = desired
        .into_iter()
        .filter(|file| {
            plan.missing.contains(&file.path)
                || plan.identity.contains(&file.path)
                || (options.force && plan.conflicts.contains(&file.path))
        })
        .map(|file| (file.path.clone(), file))
        .collect();
    if let Err(error) = apply_migration_plan(&migration)
        .and_then(|_| apply_desired_files(&target, apply_paths.values()))
        .and_then(|_| verify_and_write_metadata(&target, &variables, &snapshot))
    {
        transaction.rollback()?;
        return Err(format!("{error}\n已回滚本次 attach"));
    }
    let backup = transaction.finish();
    println!("接管完成");
    println!("目标目录：{}", target.display());
    println!("新增研发管理资产：{}", plan.missing.len());
    if !plan.identity.is_empty() {
        println!("身份转换：{}", plan.identity.len());
    }
    if !plan.conflicts.is_empty() {
        println!("force 覆盖冲突：{}", plan.conflicts.len());
    }
    println!("备份目录：{}", backup.display());
    Ok(())
}

fn run_sync(args: &[String]) -> Result<(), String> {
    let options = parse_args(args)?;
    if options.help {
        print_help();
        return Ok(());
    }
    let target = absolute_path(
        options
            .target_dir
            .as_deref()
            .unwrap_or_else(|| Path::new(".")),
    )?;
    ensure_existing_directory(&target, "sync 目标目录必须是已经存在的项目目录")?;
    let metadata_path = target.join(".yss-template.json");
    let metadata: Value = serde_json::from_slice(
        &fs::read(&metadata_path).map_err(|_| {
            "当前目录不是受支持的模板实例仓库，缺少模板元数据文件 .yss-template.json；请先使用 attach".to_owned()
        })?,
    )
    .map_err(|error| format!("模板元数据无法解析：{error}"))?;
    let metadata_version = match metadata.get("metadataSchemaVersion") {
        None => 1,
        Some(value) => value
            .as_u64()
            .filter(|version| *version > 0)
            .ok_or_else(|| "metadataSchemaVersion 必须是正整数".to_owned())?,
    };
    let legacy_metadata = metadata_version < 2;
    let missing_runtime = metadata.get("runtime").is_none();
    if !legacy_metadata {
        verify_metadata_shape(&metadata, !missing_runtime)?;
    }
    let variables = variables_from_metadata(&metadata)?;
    let snapshot = Snapshot::load()?;
    let desired = desired_for_target(&snapshot, &variables, &target)?;
    let migration = build_migration_plan(&target, &desired)?;
    let records = metadata_records(&metadata)?;
    let plan = classify_sync(&target, &desired, &records, legacy_metadata)?;
    if options.dry_run {
        let previous_version = metadata
            .get("templateVersion")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        print_sync_dry_run(&target, &plan, &migration, previous_version);
        return Ok(());
    }
    if !migration.unsafe_paths.is_empty() {
        return Err(format!(
            "sync 被 unsafe 迁移项阻断：{}",
            migration.unsafe_paths.join(", ")
        ));
    }
    if !migration.conflicts.is_empty() {
        return Err(format!(
            "sync 被旧路径迁移冲突阻断：{}",
            migration.conflicts.join("；")
        ));
    }
    if !plan.unsafe_paths.is_empty() {
        return Err(format!(
            "sync 被 unsafe 受管路径阻断：{}",
            plan.unsafe_paths.join(", ")
        ));
    }
    let mut transaction = Transaction::begin(&target)?;
    let apply_paths: BTreeMap<String, DesiredFile> = desired
        .iter()
        .filter(|file| {
            plan.updated.contains(&file.path)
                || plan.added.contains(&file.path)
                || (options.force && plan.forceable_conflicts.contains(&file.path))
        })
        .cloned()
        .map(|file| (file.path.clone(), file))
        .collect();
    if let Err(error) = apply_migration_plan(&migration)
        .and_then(|_| apply_desired_files(&target, apply_paths.values()))
        .and_then(|_| verify_instance_tree(&target))
        .and_then(|_| {
            write_sync_metadata(
                &target,
                &variables,
                &desired,
                &records,
                &plan,
                options.force,
            )
        })
    {
        transaction.rollback()?;
        return Err(format!("{error}\n已回滚本次 sync"));
    }
    let backup = transaction.finish();
    println!("同步完成");
    println!("模板版本：{CLI_VERSION}");
    println!("自动更新：{}", plan.updated.len());
    println!("新增文件：{}", plan.added.len());
    println!("跳过文件：{}", plan.skipped.len());
    println!("备份目录：{}", backup.display());
    Ok(())
}

fn run_verify_template(args: &[String]) -> Result<(), String> {
    let options = parse_args(args)?;
    if options.help {
        println!("用法：create-yss-spec verify-template [--target-dir <dir>]");
        return Ok(());
    }
    let target = absolute_path(
        options
            .target_dir
            .as_deref()
            .unwrap_or_else(|| Path::new(".")),
    )?;
    ensure_existing_directory(&target, "verify-template 目标目录必须是已经存在的项目目录")?;
    verify_instance(&target)?;
    println!("模板实例 native verify 通过");
    Ok(())
}

fn ensure_existing_directory(path: &Path, message: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| message.to_owned())?;
    if !metadata.is_dir() {
        return Err(message.to_owned());
    }
    Ok(())
}

fn desired_for_target(
    snapshot: &Snapshot,
    variables: &Variables,
    target: &Path,
) -> Result<Vec<DesiredFile>, String> {
    let mut desired = snapshot.desired_files(variables)?;
    let identity = target.join("yss-project.yaml");
    if identity.is_file() {
        let existing =
            fs::read(&identity).map_err(|error| format!("无法读取 yss-project.yaml：{error}"))?;
        if existing
            .windows(b"repository_mode: project-instance".len())
            .any(|window| window == b"repository_mode: project-instance")
            && let Some(file) = desired
                .iter_mut()
                .find(|file| file.path == "yss-project.yaml")
        {
            file.content = existing;
        }
    }
    Ok(desired)
}

#[derive(Debug, Default)]
struct AttachPlan {
    missing: Vec<String>,
    same: Vec<String>,
    identity: Vec<String>,
    conflicts: Vec<String>,
}

fn classify_attach(target: &Path, desired: &[DesiredFile]) -> Result<AttachPlan, String> {
    let mut plan = AttachPlan::default();
    for file in desired {
        let path = safe_join(target, &file.path)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                let current =
                    fs::read(&path).map_err(|error| format!("无法读取 {}：{error}", file.path))?;
                if file.path == "yss-project.yaml"
                    && current
                        .windows(b"repository_mode: template-source".len())
                        .any(|window| window == b"repository_mode: template-source")
                {
                    plan.identity.push(file.path.clone());
                    continue;
                }
                if sha256(&current) == sha256(&file.content) {
                    plan.same.push(file.path.clone());
                } else {
                    plan.conflicts.push(file.path.clone());
                }
            }
            Ok(_) => return Err(format!("目标受管路径类型不安全：{}", file.path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                plan.missing.push(file.path.clone());
            }
            Err(error) => return Err(format!("无法检查 {}：{error}", file.path)),
        }
    }
    Ok(plan)
}

fn build_migration_plan(target: &Path, desired: &[DesiredFile]) -> Result<MigrationPlan, String> {
    let desired_paths = desired
        .iter()
        .map(|file| file.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut plan = MigrationPlan::default();
    for root in AGENT_SKILL_ROOTS {
        add_migration_path(
            target,
            &format!("{root}/to-prd"),
            &format!("{root}/to-spec"),
            &desired_paths,
            &mut plan,
        )?;
        add_migration_path(
            target,
            &format!("{root}/to-issues"),
            &format!("{root}/to-tickets"),
            &desired_paths,
            &mut plan,
        )?;
    }
    add_migration_path(
        target,
        "docs/templates/prd-template.md",
        "docs/templates/spec-template.md",
        &desired_paths,
        &mut plan,
    )?;
    add_migration_path(
        target,
        "docs/templates/vertical-slice-issue-template.md",
        "docs/templates/vertical-slice-ticket-template.md",
        &desired_paths,
        &mut plan,
    )?;
    add_migration_path(
        target,
        "docs/requirements/issues",
        "docs/requirements/tickets",
        &desired_paths,
        &mut plan,
    )?;
    add_migration_path(
        target,
        ".scratch",
        "docs/.scratch",
        &desired_paths,
        &mut plan,
    )?;
    let requirements = safe_join(target, "docs/requirements")?;
    if requirements.is_dir() {
        for entry in
            fs::read_dir(&requirements).map_err(|error| format!("无法读取旧规格目录：{error}"))?
        {
            let entry = entry.map_err(|error| format!("无法读取旧规格条目：{error}"))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with("-prd.md") {
                add_migration_path(
                    target,
                    &format!("docs/requirements/{name}"),
                    &format!(
                        "docs/requirements/{}-spec.md",
                        name.trim_end_matches("-prd.md")
                    ),
                    &desired_paths,
                    &mut plan,
                )?;
            }
        }
    }
    Ok(plan)
}

fn add_migration_path(
    target: &Path,
    from: &str,
    to: &str,
    desired_paths: &std::collections::BTreeSet<&str>,
    plan: &mut MigrationPlan,
) -> Result<(), String> {
    let source = safe_join(target, from)?;
    let Ok(source_metadata) = fs::symlink_metadata(&source) else {
        return Ok(());
    };
    if source_metadata.file_type().is_symlink() {
        plan.unsafe_paths.push(from.to_owned());
        return Ok(());
    }
    let destination = safe_join(target, to)?;
    if source_metadata.is_dir() {
        match fs::symlink_metadata(&destination) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                plan.moves.push((source, destination));
                return Ok(());
            }
            Err(error) => {
                plan.conflicts.push(format!("{from} -> {to}：{error}"));
                return Ok(());
            }
            Ok(destination_metadata) if destination_metadata.file_type().is_symlink() => {
                plan.unsafe_paths.push(to.to_owned());
                return Ok(());
            }
            Ok(destination_metadata) if !destination_metadata.is_dir() => {
                plan.conflicts
                    .push(format!("{from} -> {to}：迁移目标不是目录"));
                return Ok(());
            }
            Ok(_) => {
                for entry in fs::read_dir(&source)
                    .map_err(|error| format!("无法读取旧目录 {from}：{error}"))?
                {
                    let entry = entry.map_err(|error| format!("无法读取旧目录条目：{error}"))?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    add_migration_path(
                        target,
                        &format!("{from}/{name}"),
                        &format!("{to}/{name}"),
                        desired_paths,
                        plan,
                    )?;
                }
                plan.removes.push(source);
                return Ok(());
            }
        }
    }
    let destination_managed = desired_paths
        .iter()
        .any(|path| *path == to || path.starts_with(&format!("{to}/")));
    if destination_managed {
        plan.removes.push(source);
        return Ok(());
    }
    match fs::symlink_metadata(&destination) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            plan.moves.push((source, destination));
        }
        Err(error) => plan.conflicts.push(format!("{from} -> {to}：{error}")),
        Ok(destination_metadata) if destination_metadata.file_type().is_symlink() => {
            plan.unsafe_paths.push(to.to_owned());
        }
        Ok(destination_metadata) => {
            if source_metadata.is_file() && destination_metadata.is_file() {
                let left =
                    fs::read(&source).map_err(|error| format!("无法读取旧文件 {from}：{error}"))?;
                let right = fs::read(&destination)
                    .map_err(|error| format!("无法读取迁移目标 {to}：{error}"))?;
                if left == right {
                    plan.removes.push(source);
                } else {
                    plan.conflicts
                        .push(format!("{from} -> {to}：迁移目标已存在且内容不一致"));
                }
            } else {
                plan.conflicts
                    .push(format!("{from} -> {to}：迁移目标已存在"));
            }
        }
    }
    Ok(())
}

fn apply_migration_plan(plan: &MigrationPlan) -> Result<(), String> {
    if !plan.unsafe_paths.is_empty() {
        return Err(format!(
            "legacy migration 被 unsafe 路径阻断：{}",
            plan.unsafe_paths.join(", ")
        ));
    }
    if !plan.conflicts.is_empty() {
        return Err(format!(
            "legacy migration 存在冲突：{}",
            plan.conflicts.join("；")
        ));
    }
    for (source, destination) in &plan.moves {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("无法创建迁移目录：{error}"))?;
        }
        fs::rename(source, destination)
            .map_err(|error| format!("无法迁移 {}：{error}", source.display()))?;
    }
    for source in &plan.removes {
        if source.exists() {
            remove_path(source)?;
        }
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("无法读取待删除路径：{error}"))?;
    if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|error| format!("无法删除旧目录：{error}"))
    } else {
        fs::remove_file(path).map_err(|error| format!("无法删除旧文件：{error}"))
    }
}

#[derive(Debug, Default)]
struct SyncPlan {
    updated: Vec<String>,
    added: Vec<String>,
    skipped: Vec<String>,
    forceable_conflicts: Vec<String>,
    unmanaged_conflicts: Vec<String>,
    removed: Vec<String>,
    unchanged: usize,
    unsafe_paths: Vec<String>,
}

fn classify_sync(
    target: &Path,
    desired: &[DesiredFile],
    records: &BTreeMap<String, String>,
    legacy_metadata: bool,
) -> Result<SyncPlan, String> {
    let mut plan = SyncPlan::default();
    for file in desired {
        let path = safe_join(target, &file.path)?;
        let expected = sha256(&file.content);
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                plan.added.push(file.path.clone());
            }
            Err(error) => return Err(format!("无法检查 {}：{error}", file.path)),
            Ok(metadata) if !metadata.is_file() => {
                plan.unsafe_paths.push(file.path.clone());
                continue;
            }
            Ok(_) => {
                let current_content =
                    fs::read(&path).map_err(|error| format!("无法读取 {}：{error}", file.path))?;
                let current = sha256(&current_content);
                if current == expected {
                    plan.unchanged += 1;
                    continue;
                }
                if file.path == "yss-project.yaml"
                    && current_content
                        .windows(b"repository_mode: template-source".len())
                        .any(|window| window == b"repository_mode: template-source")
                {
                    plan.updated.push(file.path.clone());
                    continue;
                }
                if legacy_metadata {
                    continue;
                }
                match records.get(&file.path) {
                    Some(baseline) if baseline == &current => plan.updated.push(file.path.clone()),
                    Some(_) => {
                        plan.skipped.push(file.path.clone());
                        plan.forceable_conflicts.push(file.path.clone());
                    }
                    None => {
                        plan.skipped.push(file.path.clone());
                        plan.unmanaged_conflicts.push(file.path.clone());
                    }
                }
            }
        }
    }
    let desired_paths = desired
        .iter()
        .map(|file| file.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    plan.removed = records
        .keys()
        .filter(|path| !desired_paths.contains(path.as_str()))
        .cloned()
        .collect();
    Ok(plan)
}

fn print_attach_dry_run(target: &Path, plan: &AttachPlan, migration: &MigrationPlan) {
    println!("attach dry-run 预览");
    println!("目标目录：{}", target.display());
    print_limited_operations(&plan.missing, |path| format!("add: {path}"));
    print_limited_operations(&plan.identity, |path| {
        format!("identity: {path}（规范化为 project-instance）")
    });
    print_limited_operations(&plan.conflicts, |path| {
        format!("conflict: {path} (目标受管文件已存在且内容不一致)")
    });
    print_migration_operations(migration, target);
    if !plan.same.is_empty() {
        println!(
            "matched: {} 项内容一致，已纳入 managed baseline",
            plan.same.len()
        );
    }
    println!(
        "统计：新增 {}，一致 {}，身份转换 {}，冲突 {}，unsafe {}",
        plan.missing.len(),
        plan.same.len(),
        plan.identity.len(),
        plan.conflicts.len(),
        migration.unsafe_paths.len()
    );
    if !plan.conflicts.is_empty() {
        println!("提示：apply 需要显式传入 --force 才能覆盖冲突受管文件");
    }
}

fn print_sync_dry_run(
    target: &Path,
    plan: &SyncPlan,
    migration: &MigrationPlan,
    previous_version: &str,
) {
    println!("sync dry-run 预览");
    println!("目标目录：{}", target.display());
    println!("模板版本：{previous_version} -> {CLI_VERSION}");
    print_limited_operations(&plan.updated, |path| format!("update: {path}"));
    print_limited_operations(&plan.added, |path| format!("add: {path}"));
    print_limited_operations(&plan.skipped, |path| {
        format!("conflict: {path} (本地修改的受管文件)")
    });
    print_limited_operations(&plan.unsafe_paths, |path| {
        format!("unsafe: {path} (目标路径类型不安全)")
    });
    print_limited_operations(&plan.removed, |path| format!("remove-report: {path}"));
    println!("unchanged: {}", plan.unchanged);
    print_migration_operations(migration, target);
}

fn print_limited_operations<F>(paths: &[String], formatter: F)
where
    F: Fn(&str) -> String,
{
    let mut sorted = paths.to_vec();
    sorted.sort_by(|left, right| {
        let left_root = !left.contains('/');
        let right_root = !right.contains('/');
        right_root.cmp(&left_root).then_with(|| left.cmp(right))
    });
    for path in sorted.iter().take(40) {
        println!("{}", formatter(path));
    }
    if sorted.len() > 40 {
        println!(
            "... 其余 {} 项省略，可用 manifest / git diff 查看完整清单",
            sorted.len() - 40
        );
    }
}

fn print_migration_operations(migration: &MigrationPlan, target: &Path) {
    for (from, to) in &migration.moves {
        println!(
            "legacy: move {} -> {}",
            relative_display(target, from),
            relative_display(target, to)
        );
    }
    for path in &migration.unsafe_paths {
        println!("unsafe: {path}");
    }
    for conflict in &migration.conflicts {
        println!("conflict: {conflict}");
    }
}

fn relative_display(target: &Path, path: &Path) -> String {
    path.strip_prefix(target)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn apply_desired_files<'a>(
    target: &Path,
    files: impl Iterator<Item = &'a DesiredFile>,
) -> Result<(), String> {
    for file in files {
        let destination = safe_join(target, &file.path)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("无法创建模板目录：{error}"))?;
        }
        fs::write(&destination, &file.content)
            .map_err(|error| format!("无法写入模板文件 {}：{error}", file.path))?;
        set_mode(&destination, file.mode)?;
    }
    Ok(())
}

fn verify_and_write_metadata(
    target: &Path,
    variables: &Variables,
    snapshot: &Snapshot,
) -> Result<(), String> {
    verify_instance_tree(target)?;
    let desired = desired_for_target(snapshot, variables, target)?;
    let managed = desired
        .iter()
        .filter_map(|file| {
            let path = safe_join(target, &file.path).ok()?;
            if path.is_file() {
                Some(ManagedFile {
                    path: file.path.clone(),
                    kind: file.kind,
                    content_hash: sha256(&fs::read(path).ok()?),
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    write_metadata(target, variables, &managed)
}

fn write_sync_metadata(
    target: &Path,
    variables: &Variables,
    desired: &[DesiredFile],
    previous: &BTreeMap<String, String>,
    plan: &SyncPlan,
    force: bool,
) -> Result<(), String> {
    let mut managed = Vec::new();
    for file in desired {
        let kept_conflict = plan.skipped.contains(&file.path)
            && !(force && plan.forceable_conflicts.contains(&file.path));
        let content_hash = if kept_conflict {
            previous.get(&file.path).cloned()
        } else {
            let path = safe_join(target, &file.path)?;
            if path.is_file() {
                Some(sha256(&fs::read(path).map_err(|error| {
                    format!("无法读取 {}：{error}", file.path)
                })?))
            } else {
                None
            }
        };
        if let Some(content_hash) = content_hash {
            managed.push(ManagedFile {
                path: file.path.clone(),
                kind: file.kind,
                content_hash,
            });
        }
    }
    write_metadata(target, variables, &managed)
}

fn verify_instance(target: &Path) -> Result<(), String> {
    verify_instance_tree(target)?;
    let metadata: Value = serde_json::from_slice(&read_regular_file(
        target,
        ".yss-template.json",
        "native verify 失败：缺少 .yss-template.json",
    )?)
    .map_err(|error| format!("native verify 失败：metadata 无法解析：{error}"))?;
    verify_metadata_shape(&metadata, true)?;
    let managed = metadata
        .get("managedFiles")
        .and_then(Value::as_object)
        .ok_or_else(|| "native verify 失败：managedFiles 缺失或非法".to_owned())?;
    let variables = variables_from_metadata(&metadata)?;
    let expected_paths = Snapshot::load()?
        .desired_files(&variables)?
        .into_iter()
        .map(|file| file.path)
        .collect::<std::collections::BTreeSet<_>>();
    let actual_paths = managed
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if actual_paths != expected_paths {
        return Err(format!(
            "native verify 失败：managedFiles 集合与当前 snapshot 不一致（期望 {}，实际 {}）",
            expected_paths.len(),
            actual_paths.len()
        ));
    }
    for (relative, descriptor) in managed {
        let kind = descriptor
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("native verify 失败：managedFiles 缺少 type：{relative}"))?;
        if kind != "copy" && kind != "render" {
            return Err(format!(
                "native verify 失败：managedFiles type 非法：{relative}"
            ));
        }
        let expected = descriptor
            .get("contentHash")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!("native verify 失败：managedFiles 缺少 contentHash：{relative}")
            })?;
        let actual = sha256(&read_regular_file(
            target,
            relative,
            &format!("native verify 失败：缺少受管文件 {relative}"),
        )?);
        if actual != expected {
            return Err(format!(
                "native verify 失败：受管文件内容 hash 不匹配：{relative}"
            ));
        }
    }
    Ok(())
}

fn verify_instance_tree(target: &Path) -> Result<(), String> {
    let identity = String::from_utf8(read_regular_file(
        target,
        "yss-project.yaml",
        "native verify 失败：缺少 yss-project.yaml",
    )?)
    .map_err(|error| format!("native verify 失败：yss-project.yaml 不是 UTF-8：{error}"))?;
    if !identity.contains("schema_version: 1")
        || !identity.contains("repository_mode: project-instance")
    {
        return Err("native verify 失败：repository_mode 必须是 project-instance".to_owned());
    }
    for required in ["AGENTS.md", "CONTEXT.md", "skills-lock.json"] {
        let _ = read_regular_file(
            target,
            required,
            &format!("native verify 失败：缺少 {required}"),
        )?;
    }
    Ok(())
}

fn read_regular_file(root: &Path, relative: &str, missing: &str) -> Result<Vec<u8>, String> {
    let path = safe_join(root, relative)?;
    let metadata = fs::symlink_metadata(&path).map_err(|_| missing.to_owned())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{missing}（路径必须是普通文件）"));
    }
    fs::read(&path).map_err(|error| format!("无法读取 {relative}：{error}"))
}

struct Transaction {
    target: PathBuf,
    backup: PathBuf,
}

impl Transaction {
    fn begin(target: &Path) -> Result<Self, String> {
        let backup = backup_path(target)?;
        copy_tree(target, &backup)?;
        Ok(Self {
            target: target.to_path_buf(),
            backup,
        })
    }

    fn rollback(&mut self) -> Result<(), String> {
        if self.target.exists() {
            fs::remove_dir_all(&self.target)
                .map_err(|error| format!("回滚目标目录失败：{error}"))?;
        }
        fs::rename(&self.backup, &self.target).map_err(|error| format!("回滚事务备份失败：{error}"))
    }

    fn finish(self) -> PathBuf {
        self.backup
    }
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(source).map_err(|error| format!("无法备份目标：{error}"))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("拒绝备份符号链接：{}", source.display()));
    }
    if metadata.is_dir() {
        fs::create_dir_all(destination).map_err(|error| format!("无法创建备份目录：{error}"))?;
        for entry in fs::read_dir(source).map_err(|error| format!("无法读取目标：{error}"))?
        {
            let entry = entry.map_err(|error| format!("无法读取目标条目：{error}"))?;
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("无法创建备份父目录：{error}"))?;
    }
    fs::copy(source, destination).map_err(|error| format!("无法备份目标文件：{error}"))?;
    set_mode(destination, mode_of(&metadata))
}

fn mode_of(metadata: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        0o644
    }
}

fn verify_metadata_shape(metadata: &Value, require_runtime: bool) -> Result<(), String> {
    let object = metadata
        .as_object()
        .ok_or_else(|| "模板元数据必须是 JSON 对象".to_owned())?;
    if object.get("metadataSchemaVersion").and_then(Value::as_u64) != Some(2) {
        return Err("不支持的模板元数据版本".to_owned());
    }
    if object.get("templateName").and_then(Value::as_str) != Some("create-yss-spec") {
        return Err("模板元数据 templateName 与当前 CLI 不匹配".to_owned());
    }
    if object
        .get("cliVersion")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err("模板元数据 cliVersion 缺失或非法".to_owned());
    }
    if object.get("templateSource").and_then(Value::as_str) != Some(TEMPLATE_SOURCE) {
        return Err("模板元数据 templateSource 缺失或非法".to_owned());
    }
    if object.get("templateCommit").and_then(Value::as_str) != Some(SNAPSHOT_COMMIT) {
        return Err("模板元数据 templateCommit 与当前 snapshot 不匹配".to_owned());
    }
    if object
        .get("managedFilesManifestVersion")
        .and_then(Value::as_str)
        != Some(TEMPLATE_MANIFEST_SHA256)
    {
        return Err("模板元数据 managedFilesManifestVersion 与当前 manifest 不匹配".to_owned());
    }
    if object
        .get("managedFiles")
        .and_then(Value::as_object)
        .is_none()
    {
        return Err("模板元数据 managedFiles 缺失或非法".to_owned());
    }
    if object.get("variables").and_then(Value::as_object).is_none() {
        return Err("模板元数据 variables 必须是 JSON 对象".to_owned());
    }
    let Some(runtime_value) = object.get("runtime") else {
        if require_runtime {
            return Err("模板元数据缺少 native runtime metadata".to_owned());
        }
        return Ok(());
    };
    let Some(runtime) = runtime_value.as_object() else {
        return Err("模板元数据 native runtime metadata 非法".to_owned());
    };
    if runtime.is_empty() {
        if require_runtime {
            return Err("模板元数据缺少 native runtime metadata".to_owned());
        }
        return Ok(());
    }
    if runtime.get("kind").and_then(Value::as_str) != Some("native-rust")
        || runtime.get("name").and_then(Value::as_str) != Some("create-yss-spec")
        || runtime.get("templateCommit").and_then(Value::as_str) != Some(SNAPSHOT_COMMIT)
        || runtime.get("snapshotHash").and_then(Value::as_str) != Some(SNAPSHOT_HASH)
        || runtime.get("snapshotArchiveSha256").and_then(Value::as_str)
            != Some(SNAPSHOT_ARCHIVE_SHA256)
    {
        return Err("模板元数据 native runtime metadata 与当前 snapshot 不匹配".to_owned());
    }
    Ok(())
}

fn variables_from_metadata(metadata: &Value) -> Result<Variables, String> {
    let variables = metadata.get("variables").and_then(Value::as_object);
    Ok(Variables {
        project_name: variables
            .and_then(|value| value.get("projectName"))
            .and_then(Value::as_str)
            .unwrap_or("待补充")
            .to_owned(),
        business_domain: variables
            .and_then(|value| value.get("businessDomain"))
            .and_then(Value::as_str)
            .unwrap_or("待补充")
            .to_owned(),
        team_size: variables
            .and_then(|value| value.get("teamSize"))
            .and_then(Value::as_str)
            .unwrap_or("待补充")
            .to_owned(),
        issue_tracker: variables
            .and_then(|value| value.get("issueTracker"))
            .and_then(Value::as_str)
            .unwrap_or("github")
            .to_owned(),
        include_example_docs: variables
            .and_then(|value| value.get("includeExampleDocs"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
    })
}

fn metadata_records(metadata: &Value) -> Result<BTreeMap<String, String>, String> {
    let mut records = BTreeMap::new();
    let Some(managed) = metadata.get("managedFiles").and_then(Value::as_object) else {
        return Ok(records);
    };
    for (path, value) in managed {
        let hash = value
            .get("contentHash")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("模板元数据 managedFiles 缺少 contentHash：{path}"))?;
        records.insert(path.clone(), hash.to_owned());
    }
    Ok(records)
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    env::current_dir()
        .map(|current| current.join(path))
        .map_err(|error| format!("无法解析目标目录：{error}"))
}

fn prepare_target(target: &Path, force: bool) -> Result<(Option<PathBuf>, bool), String> {
    if target.exists() {
        let metadata =
            fs::symlink_metadata(target).map_err(|error| format!("无法读取目标目录：{error}"))?;
        if !metadata.is_dir() {
            return Err("目标目录必须是目录".to_owned());
        }
        let non_empty = fs::read_dir(target)
            .map_err(|error| format!("无法读取目标目录：{error}"))?
            .next()
            .is_some();
        if non_empty && !force {
            return Err("目标目录非空；如需事务性替换请显式使用 --force".to_owned());
        }
        if non_empty && force {
            let backup = backup_path(target)?;
            fs::rename(target, &backup).map_err(|error| format!("无法建立事务备份：{error}"))?;
            if let Err(error) = fs::create_dir_all(target) {
                let _ = fs::rename(&backup, target);
                return Err(format!("无法创建替换目录：{error}"));
            }
            let previous_git = backup.join(".git");
            if let Ok(git_metadata) = fs::symlink_metadata(&previous_git) {
                if git_metadata.file_type().is_symlink() {
                    let _ = fs::remove_dir_all(target);
                    let _ = fs::rename(&backup, target);
                    return Err("拒绝保留符号链接形式的 .git".to_owned());
                }
                if let Err(error) = fs::rename(&previous_git, target.join(".git")) {
                    let _ = fs::remove_dir_all(target);
                    let _ = fs::rename(&backup, target);
                    return Err(format!("无法保留现有 .git：{error}"));
                }
            }
            return Ok((Some(backup), false));
        }
        return Ok((None, true));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("无法创建目标父目录：{error}"))?;
    }
    fs::create_dir_all(target).map_err(|error| format!("无法创建目标目录：{error}"))?;
    Ok((None, false))
}

fn clear_directory_contents(target: &Path) -> Result<(), String> {
    for entry in fs::read_dir(target).map_err(|error| format!("无法读取回滚目录：{error}"))?
    {
        let entry = entry.map_err(|error| format!("无法读取回滚条目：{error}"))?;
        remove_path(&entry.path())?;
    }
    Ok(())
}

fn backup_path(target: &Path) -> Result<PathBuf, String> {
    let parent = target
        .parent()
        .ok_or_else(|| "目标目录缺少可用父目录".to_owned())?;
    let name = target
        .file_name()
        .ok_or_else(|| "目标目录缺少可用名称".to_owned())?
        .to_string_lossy();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("无法生成事务标识：{error}"))?
        .as_nanos();
    Ok(parent.join(format!(".{name}.create-yss-spec-backup-{nonce}")))
}

fn restore_backup(target: &Path, backup: &Path) -> Result<(), String> {
    let active_git = target.join(".git");
    let backup_git = backup.join(".git");
    if fs::symlink_metadata(&active_git).is_ok() && fs::symlink_metadata(&backup_git).is_err() {
        fs::rename(&active_git, &backup_git)
            .map_err(|error| format!("回滚时保留 .git 失败：{error}"))?;
    }
    if target.exists() {
        fs::remove_dir_all(target).map_err(|error| format!("回滚目标目录失败：{error}"))?;
    }
    fs::rename(backup, target).map_err(|error| format!("回滚事务备份失败：{error}"))
}

fn initialize_target(
    target: &Path,
    variables: &Variables,
    snapshot: &Snapshot,
    git_init: bool,
) -> Result<(), String> {
    let managed_files = snapshot.extract(target, variables)?;
    write_metadata(target, variables, &managed_files)?;
    if git_init {
        initialize_git(target)?;
    }
    Ok(())
}

fn initialize_git(target: &Path) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .arg("init")
        .current_dir(target)
        .output()
        .map_err(|error| format!("git init 执行失败：{error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(())
}

struct Snapshot;

impl Snapshot {
    fn load() -> Result<Self, String> {
        let actual = sha256(SNAPSHOT_ARCHIVE);
        if actual != SNAPSHOT_ARCHIVE_SHA256 {
            return Err(format!(
                "模板快照 archive hash 不匹配：期望 {SNAPSHOT_ARCHIVE_SHA256}，实际 {actual}"
            ));
        }
        let mut archive = Archive::new(Cursor::new(SNAPSHOT_ARCHIVE));
        let mut manifest = None;
        let mut snapshot = None;
        for item in archive
            .entries()
            .map_err(|error| format!("模板快照无法读取：{error}"))?
        {
            let mut entry = item.map_err(|error| format!("模板快照条目无法读取：{error}"))?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let path = entry
                .path()
                .map_err(|error| format!("模板快照路径无法解析：{error}"))?
                .to_string_lossy()
                .trim_start_matches("./")
                .to_owned();
            let mut content = Vec::new();
            entry
                .read_to_end(&mut content)
                .map_err(|error| format!("模板快照绑定文件无法读取：{error}"))?;
            match path.as_str() {
                "__yss_runtime/template.manifest.json" => manifest = Some(content),
                "__yss_runtime/template.snapshot.json" => snapshot = Some(content),
                _ => {}
            }
        }
        let manifest = manifest.ok_or_else(|| "模板快照缺少 manifest 绑定文件".to_owned())?;
        if sha256(&manifest) != TEMPLATE_MANIFEST_SHA256 {
            return Err(format!(
                "模板 manifest hash 不匹配：期望 {TEMPLATE_MANIFEST_SHA256}"
            ));
        }
        let manifest: Value = serde_json::from_slice(&manifest)
            .map_err(|error| format!("模板 manifest 无法解析：{error}"))?;
        if !manifest_array_matches(&manifest, "excludeRootEntries", EXCLUDED_ROOT_ENTRIES)
            || !manifest_array_matches(&manifest, "excludeRootFiles", EXCLUDED_ROOT_FILES)
            || !manifest_array_matches(&manifest, "excludePaths", EXCLUDED_PATHS)
            || !manifest_array_matches(&manifest, "renderPaths", RENDER_PATHS)
            || !manifest_array_matches(&manifest, "exampleDocPaths", &[EXAMPLE_DOC_PATH])
        {
            return Err("模板 manifest 与 runtime 不匹配".to_owned());
        }
        let snapshot = snapshot.ok_or_else(|| "模板快照缺少 snapshot 绑定文件".to_owned())?;
        let snapshot: Value = serde_json::from_slice(&snapshot)
            .map_err(|error| format!("模板 snapshot 无法解析：{error}"))?;
        if snapshot.get("schemaVersion").and_then(Value::as_u64) != Some(1)
            || snapshot.get("templateName").and_then(Value::as_str)
                != Some("yss-spec-project-template")
            || snapshot.get("templateSource").and_then(Value::as_str) != Some(TEMPLATE_SOURCE)
            || snapshot.get("requestedRef").and_then(Value::as_str) != Some(SNAPSHOT_COMMIT)
            || snapshot.get("templateCommit").and_then(Value::as_str) != Some(SNAPSHOT_COMMIT)
            || snapshot.get("snapshotHash").and_then(Value::as_str) != Some(SNAPSHOT_HASH)
        {
            return Err("模板 snapshot commit/hash 与 runtime 不匹配".to_owned());
        }
        let expected_encoded_paths = json!({
            ".codex/skills/data-analytics/.gitignore": ".codex/skills/data-analytics/__yss_dotfile__.gitignore",
            ".codex/skills/product-design/.npmignore": ".codex/skills/product-design/__yss_dotfile__.npmignore",
            ".codex/skills/product-design/templates/prototype/.npmrc": ".codex/skills/product-design/templates/prototype/__yss_dotfile__.npmrc",
            ".gitignore": "__yss_dotfile__.gitignore"
        });
        if snapshot.get("encodedPaths") != Some(&expected_encoded_paths) {
            return Err("模板 snapshot encodedPaths 与 runtime 不匹配".to_owned());
        }
        Ok(Self)
    }

    fn files(&self) -> Result<Vec<String>, String> {
        let mut archive = Archive::new(Cursor::new(SNAPSHOT_ARCHIVE));
        let mut paths = Vec::new();
        for item in archive
            .entries()
            .map_err(|error| format!("模板快照无法读取：{error}"))?
        {
            let entry = item.map_err(|error| format!("模板快照条目无法读取：{error}"))?;
            if entry.header().entry_type().is_dir() {
                continue;
            }
            if !entry.header().entry_type().is_file() {
                return Err("模板快照包含不支持的符号链接或特殊文件".to_owned());
            }
            let raw = entry
                .path()
                .map_err(|error| format!("模板快照路径无法解析：{error}"))?;
            let Some(path) = logical_path(&raw)? else {
                continue;
            };
            if should_exclude(&path) {
                continue;
            }
            paths.push(path);
        }
        paths.sort();
        Ok(paths)
    }

    fn desired_files(&self, variables: &Variables) -> Result<Vec<DesiredFile>, String> {
        let mut archive = Archive::new(Cursor::new(SNAPSHOT_ARCHIVE));
        let mut desired = Vec::new();
        for item in archive
            .entries()
            .map_err(|error| format!("模板快照无法读取：{error}"))?
        {
            let mut entry = item.map_err(|error| format!("模板快照条目无法读取：{error}"))?;
            if entry.header().entry_type().is_dir() {
                continue;
            }
            if !entry.header().entry_type().is_file() {
                return Err("模板快照包含不支持的符号链接或特殊文件".to_owned());
            }
            let raw = entry
                .path()
                .map_err(|error| format!("模板快照路径无法解析：{error}"))?;
            let Some(path) = logical_path(&raw)? else {
                continue;
            };
            if should_exclude(&path)
                || (!variables.include_example_docs && path == EXAMPLE_DOC_PATH)
            {
                continue;
            }
            let mut content = Vec::new();
            entry
                .read_to_end(&mut content)
                .map_err(|error| format!("模板快照内容无法读取：{error}"))?;
            let kind = if RENDER_PATHS.contains(&path.as_str()) {
                content = render(&path, &content, variables)?;
                "render"
            } else {
                "copy"
            };
            desired.push(DesiredFile {
                path,
                kind,
                content,
                mode: entry.header().mode().unwrap_or(0o644),
            });
        }
        desired.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(desired)
    }

    fn extract(&self, target: &Path, variables: &Variables) -> Result<Vec<ManagedFile>, String> {
        let desired = self.desired_files(variables)?;
        let mut managed = Vec::new();
        for file in desired {
            let destination = safe_join(target, &file.path)?;
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| format!("无法创建模板目录：{error}"))?;
            }
            fs::write(&destination, &file.content)
                .map_err(|error| format!("无法写入模板文件 {}：{error}", file.path))?;
            set_mode(&destination, file.mode)?;
            managed.push(ManagedFile {
                path: file.path,
                kind: file.kind,
                content_hash: sha256(&file.content),
            });
        }
        Ok(managed)
    }
}

fn manifest_array_matches(manifest: &Value, key: &str, expected: &[&str]) -> bool {
    manifest.get(key).and_then(Value::as_array)
        == Some(
            &expected
                .iter()
                .map(|value| Value::String((*value).to_owned()))
                .collect::<Vec<_>>(),
        )
}

fn logical_path(raw: &Path) -> Result<Option<String>, String> {
    let mut segments = Vec::new();
    for component in raw.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(value) => segments.push(value.to_string_lossy().to_string()),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("模板快照包含越界路径：{}", raw.display()));
            }
        }
    }
    if segments.is_empty() {
        return Ok(None);
    }
    if segments[0] == SNAPSHOT_BINDING_ROOT
        || EXCLUDED_ROOT_ENTRIES.contains(&segments[0].as_str())
        || (segments.len() == 1 && EXCLUDED_ROOT_FILES.contains(&segments[0].as_str()))
    {
        return Ok(None);
    }
    let encoded = segments.join("/");
    let decoded = match encoded.as_str() {
        ".codex/skills/data-analytics/__yss_dotfile__.gitignore" => {
            ".codex/skills/data-analytics/.gitignore"
        }
        ".codex/skills/product-design/__yss_dotfile__.npmignore" => {
            ".codex/skills/product-design/.npmignore"
        }
        ".codex/skills/product-design/templates/prototype/__yss_dotfile__.npmrc" => {
            ".codex/skills/product-design/templates/prototype/.npmrc"
        }
        "__yss_dotfile__.gitignore" => ".gitignore",
        _ => encoded.as_str(),
    };
    Ok(Some(decoded.to_owned()))
}

fn should_exclude(path: &str) -> bool {
    EXCLUDED_PATHS.contains(&path)
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("模板路径越界：{relative}"));
    }
    let joined = root.join(path);
    let mut current = root.to_path_buf();
    for component in path.components() {
        let Component::Normal(value) = component else {
            continue;
        };
        current.push(value);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!("路径包含不安全符号链接：{}", current.display()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(format!("无法检查路径 {}：{error}", current.display())),
        }
    }
    Ok(joined)
}

fn render(path: &str, source: &[u8], variables: &Variables) -> Result<Vec<u8>, String> {
    let content = String::from_utf8(source.to_vec())
        .map_err(|error| format!("模板文件 {path} 不是 UTF-8：{error}"))?;
    let rendered = match path {
        "yss-project.yaml" => {
            if !content.contains("repository_mode: template-source") {
                return Err(
                    "模板 yss-project.yaml 必须声明 repository_mode: template-source".to_owned(),
                );
            }
            content.replace(
                "repository_mode: template-source",
                "repository_mode: project-instance",
            )
        }
        "AGENTS.md" => content
            .replace(
                "**项目名称：** [填写]",
                &format!("**项目名称：** {}", variables.project_name),
            )
            .replace(
                "**业务领域：** [填写]",
                &format!("**业务领域：** {}", variables.business_domain),
            )
            .replace(
                "**团队规模：** [填写]",
                &format!("**团队规模：** {}", variables.team_size),
            ),
        "README.md" => {
            let mut value = content
                .replace(
                    "# YSS Spec Project Template",
                    &format!("# {}", variables.project_name),
                )
                .replace(
                    "> Matt Pocock Engineering Skills",
                    &format!(
                        "> 默认 Issue Tracker：{}\n>\n> Matt Pocock Engineering Skills",
                        variables.issue_tracker
                    ),
                );
            if !variables.include_example_docs {
                value = value
                    .lines()
                    .filter(|line| !line.contains("docs/discovery/IDEATION.md"))
                    .collect::<Vec<_>>()
                    .join("\n");
                value.push('\n');
            }
            value
        }
        _ => content,
    };
    Ok(rendered.into_bytes())
}

fn write_metadata(
    target: &Path,
    variables: &Variables,
    managed: &[ManagedFile],
) -> Result<(), String> {
    let mut managed_values = Map::new();
    for file in managed {
        managed_values.insert(
            file.path.clone(),
            json!({"type": file.kind, "contentHash": file.content_hash}),
        );
    }
    let timestamp = now_iso();
    let metadata = json!({
        "metadataSchemaVersion": 2,
        "templateName": "create-yss-spec",
        "cliVersion": CLI_VERSION,
        "templateVersion": CLI_VERSION,
        "templateSource": TEMPLATE_SOURCE,
        "templateCommit": SNAPSHOT_COMMIT,
        "initializedAt": timestamp,
        "lastSyncedAt": timestamp,
        "managedFilesManifestVersion": TEMPLATE_MANIFEST_SHA256,
        "variables": {
            "projectName": variables.project_name,
            "businessDomain": variables.business_domain,
            "teamSize": variables.team_size,
            "issueTracker": variables.issue_tracker,
            "includeExampleDocs": variables.include_example_docs
        },
        "runtime": {
            "kind": "native-rust",
            "name": "create-yss-spec",
            "version": CLI_VERSION,
            "templateCommit": SNAPSHOT_COMMIT,
            "snapshotHash": SNAPSHOT_HASH,
            "snapshotArchiveSha256": SNAPSHOT_ARCHIVE_SHA256
        },
        "managedFiles": Value::Object(managed_values)
    });
    let mut content = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| format!("模板元数据无法序列化：{error}"))?;
    content.push(b'\n');
    let metadata_path = safe_join(target, ".yss-template.json")?;
    if let Ok(metadata) = fs::symlink_metadata(&metadata_path)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err("拒绝覆盖不安全的模板元数据路径".to_owned());
    }
    fs::write(metadata_path, content).map_err(|error| format!("无法写入模板元数据：{error}"))
}

fn now_iso() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let days = (seconds / 86_400) as i64;
    let remaining = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.000Z",
        remaining / 3_600,
        (remaining % 3_600) / 60,
        remaining % 60
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

fn sha256(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o777))
            .map_err(|error| format!("无法设置文件权限 {}：{error}", path.display()))?;
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
}

fn print_help() {
    print!("{NODE_2_HELP}");
}
