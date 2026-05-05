param(
    [string] $OutputDir = "target\package-registry-smoke",
    [string] $Registry = "registry"
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$repoCargoConfig = Join-Path $repoRoot ".cargo\config.toml"

function Resolve-RepoPath {
    param([string] $Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
}

function Resolve-TargetDir {
    if ($env:CARGO_TARGET_DIR) {
        return $env:CARGO_TARGET_DIR
    }

    if (Test-Path $repoCargoConfig) {
        $configText = Get-Content $repoCargoConfig -Raw -Encoding utf8
        if ($configText -match 'target-dir\s*=\s*"([^"]+)"') {
            return $matches[1]
        }
    }

    return Join-Path $repoRoot "target"
}

function Resolve-AxcBinary {
    $overrideCandidates = @()
    if (-not [string]::IsNullOrWhiteSpace($env:AXC_BINARY)) {
        $overrideCandidates += [string] $env:AXC_BINARY
    }
    if (-not [string]::IsNullOrWhiteSpace($env:CARGO_BIN_EXE_axc)) {
        $overrideCandidates += [string] $env:CARGO_BIN_EXE_axc
    }

    foreach ($candidate in $overrideCandidates) {
        if (Test-Path $candidate) {
            return $candidate
        }
    }

    $targetDir = Resolve-TargetDir
    return Join-Path $targetDir "debug\axc.exe"
}

function Ensure-AxcBinary {
    $binary = Resolve-AxcBinary
    if (Test-Path $binary) {
        return $binary
    }

    & $cargoScript build --bin axc | Out-Null

    $binary = Resolve-AxcBinary
    if (-not (Test-Path $binary)) {
        Write-Error "Could not find compiled AX binary after build: $binary"
    }

    return $binary
}

function Write-Utf8NoBom {
    param(
        [string] $Path,
        [string] $Text
    )

    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Text, $encoding)
}

function Assert-Equal {
    param(
        [string] $Label,
        $Actual,
        $Expected
    )

    if ($Actual -ne $Expected) {
        Write-Error "$Label expected '$Expected' but got '$Actual'."
    }
}

$outputRoot = Resolve-RepoPath -Path $OutputDir
$registryRoot = Resolve-RepoPath -Path $Registry

if (Test-Path $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}

New-Item -ItemType Directory -Force -Path (Join-Path $outputRoot "src") | Out-Null

$manifestText = @'
manifest_version = 1

[package]
name = "package_registry_smoke"
entry = "src/main.ax"
sources = ["../../std"]
'@

$sourceText = @'
import api_tools.response;
import auth_tools.headers;
import bytes_tools.core;
import cache_tools.keys;
import collection_tools.ints;
import config_rules.validate;
import database_tools.dsn;
import encoding_tools.core;
import feature_flag_tools.flags;
import hash_tools.checksum;
import health_tools.checks;
import json_tools.encode;
import jwt_tools.preview;
import log_tools.core;
import markdown_tools.headings;
import math_rules.core;
import migration_tools.plan;
import number_tools.core;
import observability_tools.signals;
import pagination_tools.core;
import queue_tools.jobs;
import rate_limit_tools.window;
import report_tools.builder;
import result_tools.summary;
import retry_tools.policy;
import schema_tools.describe;
import text_tools.normalize;
import text_tools.stats;
import url_tools.core;
import validation_tools.rules;

fn main() -> i32 {
    let normalized: string = text_tools.normalize.normalize_document("# Title\n\n\tbody");
    let summary: text_tools.stats.TextSummary = text_tools.stats.analyze(normalized);
    let score: i32 = math_rules.core.score(summary.nonempty);
    let values: [i32] = [score, summary.headings, summary.nonempty];
    let totals: collection_tools.ints.IntSummary = collection_tools.ints.summarize(values);
    let bounded: number_tools.core.RangeCheck = number_tools.core.range_check(totals.sum, 0, 20);
    let headings: markdown_tools.headings.HeadingSummary = markdown_tools.headings.summarize(normalized);
    let database_status: database_tools.dsn.DsnCheck = database_tools.dsn.check("postgres://db.example/app");
    let url_status: url_tools.core.UrlSummary = url_tools.core.summarize("https://api.example/v1");
    let json: string = json_tools.encode.object3(
        json_tools.encode.field_string("service", "ax-pkg"),
        json_tools.encode.field_i32("score", score),
        json_tools.encode.field_bool("secure", url_status.secure)
    );
    let bytes_hex: string = bytes_tools.core.utf8_hex("AX");
    let base64_text: string = encoding_tools.core.base64_encode_text("AX");
    let decoded: encoding_tools.core.DecodeResult = encoding_tools.core.hex_decode("4158");
    let checksum: hash_tools.checksum.Checksum = hash_tools.checksum.summarize("AX-PKG");
    let jwt: jwt_tools.preview.JwtPreview = jwt_tools.preview.preview_subject_scope("user-1", "read");
    let retry_policy: retry_tools.policy.RetryPolicy = retry_tools.policy.exponential(4, 100, 1000);
    let page: pagination_tools.core.PageWindow = pagination_tools.core.window(52, 2, 20);
    let cache_policy: cache_tools.keys.CachePolicy = cache_tools.keys.with_stale(60, 30);
    let api_status: api_tools.response.ApiStatus = api_tools.response.ok("ready");
    let job: queue_tools.jobs.JobState = queue_tools.jobs.failed(3, 3);
    let migration: migration_tools.plan.MigrationBatch = migration_tools.plan.batch("release-001", 2, 1);
    let field: schema_tools.describe.FieldSpec = schema_tools.describe.field("id", "i32", false, true);
    let table: schema_tools.describe.TableSummary = schema_tools.describe.table("users", 4, 1);
    let metric: observability_tools.signals.MetricPoint = observability_tools.signals.duration_ms("request", 245);
    let rate_limit: rate_limit_tools.window.RateLimit = rate_limit_tools.window.create(100, 83, 60);
    let flag: feature_flag_tools.flags.FeatureFlag = feature_flag_tools.flags.flag("new-api", true, 50);
    let health: health_tools.checks.HealthSummary = health_tools.checks.summary(3, 1);
    let log_line: string = log_tools.core.info("registry-smoke", "packages loaded");
    let auth_preview: string = auth_tools.headers.safe_header_preview("Authorization", "secret-token");
    let config_status: i32 = config_rules.validate.validate("host=localhost\nport=8080\n");
    let name_status: i32 = validation_tools.rules.require_prefix("AX-PKG", "AX");
    let mut report: string = "";
    report = report_tools.builder.section(report, "registry package preview");
    report = report_tools.builder.kv_i32(report, "score", score);
    report = report_tools.builder.kv_i32(report, "sum", totals.sum);
    report = report_tools.builder.kv_string(report, "band", number_tools.core.score_band(score));
    report = report_tools.builder.kv_bool(report, "range-ok", bounded.ok);
    report = report_tools.builder.kv_i32(report, "headings", headings.headings);
    report = report_tools.builder.kv_string(report, "db", database_status.driver);
    report = report_tools.builder.kv_string(report, "url", url_status.scheme);
    report = report_tools.builder.kv_string(report, "json", json);
    report = report_tools.builder.kv_string(report, "bytes", bytes_hex);
    report = report_tools.builder.kv_string(report, "base64", base64_text);
    report = report_tools.builder.kv_bool(report, "hex-ok", decoded.ok);
    report = report_tools.builder.kv_string(report, "checksum", checksum.label);
    report = report_tools.builder.kv_bool(report, "jwt-safe", jwt.safe_for_auth);
    report = report_tools.builder.kv_string(report, "retry", retry_tools.policy.action_label(503, 1, retry_policy));
    report = report_tools.builder.kv_i32(report, "page-offset", page.offset);
    report = report_tools.builder.kv_string(report, "cache", cache_tools.keys.age_label(75, cache_policy));
    report = report_tools.builder.kv_string(report, "api", api_tools.response.status_label(api_status));
    report = report_tools.builder.kv_string(report, "queue", queue_tools.jobs.dead_letter_reason(job));
    report = report_tools.builder.kv_string(report, "migration", migration_tools.plan.batch_status(migration));
    report = report_tools.builder.kv_string(report, "field", schema_tools.describe.field_label(field));
    report = report_tools.builder.kv_string(report, "schema", schema_tools.describe.table_health(table));
    report = report_tools.builder.kv_string(report, "metric", observability_tools.signals.metric_line(metric));
    report = report_tools.builder.kv_string(report, "rate-limit", rate_limit_tools.window.status(rate_limit));
    report = report_tools.builder.kv_string(report, "flag", feature_flag_tools.flags.decision_label(flag, "user-1"));
    report = report_tools.builder.kv_string(report, "health", health.status);
    report = report_tools.builder.kv_string(report, "log", log_line);
    report = report_tools.builder.kv_string(report, "auth", auth_preview);
    report = report_tools.builder.kv_string(report, "name", validation_tools.rules.message(name_status, "name"));
    println(result_tools.summary.status_label(config_status, "ok"));
    println(report);
    return result_tools.summary.exit_code(config_status);
}
'@

Write-Utf8NoBom -Path (Join-Path $outputRoot "AX.toml") -Text $manifestText
Write-Utf8NoBom -Path (Join-Path $outputRoot "src\main.ax") -Text $sourceText

$axcBinary = Ensure-AxcBinary

& $axcBinary pkg check --registry $registryRoot
Assert-Equal -Label "axc pkg check exit code" -Actual $LASTEXITCODE -Expected 0

$maturityCases = @(
    @{ Package = "json_tools"; Expected = "maturity: stable_pure_ax" },
    @{ Package = "http_tools"; Expected = "maturity: host_boundary_preview" },
    @{ Package = "jwt_tools"; Expected = "maturity: future_native_preview" }
)
foreach ($case in $maturityCases) {
    $infoOutput = @(& $axcBinary pkg info $case.Package --registry $registryRoot | ForEach-Object { [string] $_ })
    Assert-Equal -Label "axc pkg info $($case.Package) exit code" -Actual $LASTEXITCODE -Expected 0
    if (-not $infoOutput.Contains($case.Expected)) {
        Write-Error "pkg info $($case.Package) did not include expected maturity line '$($case.Expected)'."
    }
}

$packages = @(
    "api_tools",
    "auth_tools",
    "bytes_tools",
    "cache_tools",
    "collection_tools",
    "config_rules",
    "database_tools",
    "encoding_tools",
    "feature_flag_tools",
    "hash_tools",
    "health_tools",
    "json_tools",
    "jwt_tools",
    "log_tools",
    "markdown_tools",
    "math_rules",
    "migration_tools",
    "number_tools",
    "observability_tools",
    "pagination_tools",
    "queue_tools",
    "rate_limit_tools",
    "report_tools",
    "result_tools",
    "retry_tools",
    "schema_tools",
    "text_tools",
    "url_tools",
    "validation_tools"
)

foreach ($package in $packages) {
    & $axcBinary pkg add $package $outputRoot --registry $registryRoot
    Assert-Equal -Label "axc pkg add $package exit code" -Actual $LASTEXITCODE -Expected 0
}

& $axcBinary pkg install $outputRoot --registry $registryRoot
Assert-Equal -Label "axc pkg install exit code" -Actual $LASTEXITCODE -Expected 0

$lockfilePath = Join-Path $outputRoot "AX.lock"
if (-not (Test-Path $lockfilePath)) {
    Write-Error "Package registry smoke did not produce AX.lock at $lockfilePath"
}

$lockfile = Get-Content $lockfilePath -Raw -Encoding utf8 | ConvertFrom-Json
Assert-Equal -Label "AX.lock schema_version" -Actual ([int] $lockfile.schema_version) -Expected 2
Assert-Equal -Label "AX.lock dependency count" -Actual (@($lockfile.dependencies).Count) -Expected 29

& $axcBinary check $outputRoot
Assert-Equal -Label "axc check exit code" -Actual $LASTEXITCODE -Expected 0

$runOutput = & $axcBinary run $outputRoot
Assert-Equal -Label "axc run exit code" -Actual $LASTEXITCODE -Expected 0

$expectedOutput = @(
    "ok",
    "== registry package preview ==",
    "score: 9",
    "sum: 12",
    "band: excellent",
    "range-ok: true",
    "headings: 1",
    "db: postgres",
    "url: https",
    "json: {""service"":""ax-pkg"",""score"":9,""secure"":true}",
    "bytes: 4158",
    "base64: QVg=",
    "hex-ok: true",
    "checksum: axh1:943584527",
    "jwt-safe: false",
    "retry: retry",
    "page-offset: 20",
    "cache: stale",
    "api: ok:ready",
    "queue: attempts-exhausted",
    "migration: requires-review",
    "field: id:i32#",
    "schema: ok",
    "metric: request=245ms",
    "rate-limit: near-limit",
    "flag: disabled",
    "health: degraded",
    "log: [info] registry-smoke: packages loaded",
    "auth: Authorization: <redacted:12>",
    "name: name: ok"
)
$actualOutput = @($runOutput | ForEach-Object { [string] $_ })
while ($actualOutput.Count -gt 0 -and $actualOutput[$actualOutput.Count - 1] -eq "") {
    if ($actualOutput.Count -eq 1) {
        $actualOutput = @()
    } else {
        $actualOutput = @($actualOutput[0..($actualOutput.Count - 2)])
    }
}
Assert-Equal -Label "run output line count" -Actual $actualOutput.Count -Expected $expectedOutput.Count
for ($index = 0; $index -lt $expectedOutput.Count; $index += 1) {
    Assert-Equal -Label "run output[$index]" -Actual $actualOutput[$index] -Expected $expectedOutput[$index]
}

Write-Host "Package registry smoke passed. Verified 29 stable registry packages at $outputRoot"
