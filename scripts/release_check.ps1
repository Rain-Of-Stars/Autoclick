$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [string]$Command
    )

    Write-Host ""
    Write-Host "==> $Name" -ForegroundColor Cyan
    Invoke-Expression $Command
    if ($LASTEXITCODE -ne 0) {
        throw "步骤失败: $Name"
    }
}

function Show-FailureSuggestions {
    Write-Host ""
    Write-Host "发布检查失败。建议优先排查以下项目：" -ForegroundColor Red
    Write-Host "1. 先执行 cargo check --workspace，确认是否存在编译或特性开关问题。"
    Write-Host "2. 再执行 cargo test --workspace，定位失败测试所属 crate 和具体断言。"
    Write-Host "3. 若前端失败，执行 pnpm test 与 pnpm build，确认路由、store 和命令契约是否一致。"
    Write-Host "4. 若打包失败，检查 src-tauri/tauri.conf.json、capabilities 和图标资源路径是否正确。"
    Write-Host "5. 若旧版导入失败，检查 tests/fixtures/legacy_project 或真实旧工程目录是否完整。"
}

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

try {
    Invoke-Step -Name "Rust 格式检查" -Command "cargo fmt --all --check"
    Invoke-Step -Name "Rust 静态检查" -Command "cargo clippy --workspace --all-targets -- -D warnings"
    Invoke-Step -Name "Rust 全量测试" -Command "cargo test --workspace"
    Invoke-Step -Name "Windows 集成测试" -Command "cargo test -p autoclick-tauri2 --test windows_integration -- --nocapture"
    Invoke-Step -Name "诊断包导出测试" -Command "cargo test -p autoclick-diagnostics export_bundle"
    Invoke-Step -Name "性能预算测试" -Command "cargo test -p autoclick-diagnostics perf_budget"
    Invoke-Step -Name "旧版导入验证" -Command "cargo test -p autoclick-storage import_legacy -- --nocapture"
    Invoke-Step -Name "前端冒烟测试" -Command "pnpm test"
    Invoke-Step -Name "前端构建" -Command "pnpm build"

    Write-Host ""
    Write-Host "==> 基准摘要" -ForegroundColor Cyan
    $benchFiles = Get-ChildItem -Path ".\src-tauri\crates" -Recurse -Filter "*.rs" |
        Where-Object { $_.FullName -like "*\benches\*" } |
        Select-Object -ExpandProperty FullName
    if ($benchFiles) {
        $benchFiles | ForEach-Object { Write-Host "bench: $($_.Replace($repoRoot + '\', ''))" }
        Invoke-Step -Name "基准编译检查" -Command "cargo bench --workspace --no-run"
    } else {
        Write-Host "未发现 bench 文件，跳过。"
    }

    Invoke-Step -Name "Tauri 调试构建" -Command "pnpm tauri build --debug"

    Write-Host ""
    Write-Host "发布检查通过。" -ForegroundColor Green
    exit 0
}
catch {
    Write-Host ""
    Write-Host $_.Exception.Message -ForegroundColor Red
    Show-FailureSuggestions
    exit 1
}
