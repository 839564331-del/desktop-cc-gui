<#
  .SYNOPSIS
    C 盘清理脚本（逐项确认）
  .用法
    管理员 PowerShell 里运行（推荐，可多清 休眠6.3G + WinSxS）：
      powershell -NoProfile -ExecutionPolicy Bypass -File "D:\AI-project\desktop-cc-gui-main\cleanup-c-drive.ps1"
    非管理员也能跑缓存/临时类清理（休眠/WinSxS/VS缓存 会自动跳过）。
  .交互
    每项显示大小后问： y=删 / 回车或n=跳过 / a=此项及后续全删 / q=立即退出
  .说明
    仅删缓存/临时/系统组件；不自动删 Tencent 聊天数据（结尾提示应用内清理）。
    中文若乱码，改用 PowerShell 7： pwsh -NoProfile -ExecutionPolicy Bypass -File "...\cleanup-c-drive.ps1"
#>

$ErrorActionPreference = 'SilentlyContinue'
try { chcp 65001 > $null } catch {}
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

# ---- 是否管理员 ----
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltinRole]::Administrator)

# ---- 工具函数 ----
function Get-SizeGB($path) {
    if (-not (Test-Path -LiteralPath $path)) { return $null }
    $sum = (Get-ChildItem -LiteralPath $path -Recurse -File -Force -EA SilentlyContinue | Measure-Object Length -Sum).Sum
    if (-not $sum -or $sum -le 0) { return 0 }
    return [math]::Round($sum / 1GB, 2)
}
function fmt($gb) {
    if ($null -eq $gb)  { return '(缺失)' }
    if ($gb -eq 0)      { return '  ~0' }
    if ($gb -ge 1)      { return ('{0:N2} GB' -f $gb) }
    return ('{0:N0} MB' -f ($gb * 1024))
}
function Get-FreeGB { $d = Get-PSDrive C; return [math]::Round($d.Free / 1GB, 1) }
function Is-Running($names) {
    foreach ($n in $names) { if (Get-Process -Name $n -EA SilentlyContinue) { return $true } }
    return $false
}
# 计算某项当前占用（删前/删后各调一次，差值=释放量）
function Calc-Item($it) {
    switch ($it.Kind) {
        { $_ -in @('delete','delete-children','npm') } {
            $total = 0; $any = $false
            foreach ($p in $it.Paths) { $g = Get-SizeGB $p; if ($null -ne $g) { $any = $true; $total += $g } }
            if ($any) { return [math]::Round($total, 2) } else { return $null }
        }
        'updater-glob' {
            $total = 0; $any = $false
            Get-ChildItem -LiteralPath $env:LOCALAPPDATA -Directory -Filter '*updater*' -Force -EA SilentlyContinue | ForEach-Object {
                $g = Get-SizeGB $_.FullName; if ($null -ne $g) { $any = $true; $total += $g }
            }
            if ($any) { return [math]::Round($total, 2) } else { return $null }
        }
        'hibernate' { $f = Get-Item 'C:\hiberfil.sys' -Force -EA SilentlyContinue; if ($f) { return [math]::Round($f.Length / 1GB, 2) } else { return $null } }
        'dism'      { return 'WinSxS(部分)' }
        default     { return $null }
    }
}

# ---- 清理项 ----
$items = @(
    [pscustomobject]@{ N='01'; Label='系统休眠文件 hiberfil.sys'; Kind='hibernate'; Admin=$true;  BlockIf=$null; Paths=$null; Note='关闭休眠(含快速启动)并删除，最大单项' }
    [pscustomobject]@{ N='02'; Label='Chrome 缓存 (Service Worker + Code Cache)'; Kind='delete'; Admin=$false; BlockIf=@('chrome'); Paths=@("$env:LOCALAPPDATA\Google\Chrome\User Data\Profile 1\Service Worker","$env:LOCALAPPDATA\Google\Chrome\User Data\Profile 1\Code Cache"); Note='需先完全退出 Chrome；不丢登录' }
    [pscustomobject]@{ N='03'; Label='Trae CN 全部数据'; Kind='delete'; Admin=$false; BlockIf=$null; Paths=@("$env:APPDATA\Trae CN"); Note='不用 Trae(AI IDE) 才删' }
    [pscustomobject]@{ N='04'; Label='WinSxS 旧系统组件'; Kind='dism'; Admin=$true;  BlockIf=$null; Paths=$null; Note='DISM 清理，耗时数分钟' }
    [pscustomobject]@{ N='05'; Label='剪映 JianyingPro 缓存'; Kind='delete'; Admin=$false; BlockIf=@('JianyingPro'); Paths=@("$env:LOCALAPPDATA\JianyingPro\User Data\Cache"); Note='需先关闭剪映' }
    [pscustomobject]@{ N='06'; Label='用户 Temp 临时文件'; Kind='delete-children'; Admin=$false; BlockIf=$null; Paths=@("$env:LOCALAPPDATA\Temp"); Note='被占用的文件自动跳过' }
    [pscustomobject]@{ N='07'; Label='npm 缓存'; Kind='npm'; Admin=$false; BlockIf=$null; Paths=@("$env:LOCALAPPDATA\npm-cache") }
    [pscustomobject]@{ N='08'; Label='Chrome 扩展更新缓存 component_crx_cache'; Kind='delete'; Admin=$false; BlockIf=@('chrome'); Paths=@("$env:LOCALAPPDATA\Google\Chrome\User Data\component_crx_cache") }
    [pscustomobject]@{ N='09'; Label='ima.copilot 缓存'; Kind='delete'; Admin=$false; BlockIf=$null; Paths=@("$env:LOCALAPPDATA\ima.copilot\User Data\Default\Code Cache") }
    [pscustomobject]@{ N='10'; Label='Electron updater 残留'; Kind='updater-glob'; Admin=$false; BlockIf=$null; Paths=$null; Note='Local 下 *updater* 目录，应用仍可用，更新时重建' }
    [pscustomobject]@{ N='11'; Label='VS 组件包缓存 Packages\Cache'; Kind='delete'; Admin=$true;  BlockIf=$null; Paths=@("C:\ProgramData\Microsoft\VisualStudio\Packages\Cache"); Note='删后 VS 增删组件需重下；不影响已装组件/编译' }
)

# ---- 主流程 ----
Write-Host ""
Write-Host "=== C 盘清理（逐项确认）===" -ForegroundColor Cyan
Write-Host ("管理员模式: " + $(if ($isAdmin) { '是  [可清 休眠 + WinSxS + VS缓存]' } else { '否  [休眠/WinSxS/VS缓存 将跳过，建议管理员重开]' })) -ForegroundColor $(if ($isAdmin) { 'Green' } else { 'Yellow' })
$before = Get-FreeGB
Write-Host ("C 盘剩余: $before GB")
Write-Host "选项: y=删  回车/n=跳过  a=剩下全删  q=退出"

$autoYes = $false
$totalFreed = 0.0

foreach ($it in $items) {
    $sizeBefore = Calc-Item $it

    Write-Host ""
    Write-Host ("[" + $it.N + "] " + $it.Label) -ForegroundColor White
    if ($it.Kind -eq 'updater-glob') {
        Write-Host ("     匹配: " + $env:LOCALAPPDATA + "\*updater*") -ForegroundColor DarkGray
    } elseif ($it.Paths) {
        Write-Host ("     路径: " + ($it.Paths -join ' | ')) -ForegroundColor DarkGray
    }
    if ($it.Note) { Write-Host ("     说明: " + $it.Note) -ForegroundColor DarkGray }
    $sizeStr = if ($null -eq $sizeBefore) { '(不存在/已清)' } elseif ($sizeBefore -is [string]) { $sizeBefore } else { (fmt $sizeBefore) }
    Write-Host ("     大小: " + $sizeStr)
    if ($it.Kind -eq 'updater-glob') {
        $m = Get-ChildItem -LiteralPath $env:LOCALAPPDATA -Directory -Filter '*updater*' -Force -EA SilentlyContinue | Select-Object -ExpandProperty Name
        if ($m) { Write-Host ("     匹配到: " + ($m -join ', ')) -ForegroundColor DarkGray }
    }

    # 跳过条件
    if (-not $isAdmin -and $it.Admin) { Write-Host "     -> 跳过（需管理员）" -ForegroundColor Yellow; continue }
    if ($null -eq $sizeBefore)        { Write-Host "     -> 跳过（不存在或已清）" -ForegroundColor DarkGray; continue }
    if ($it.BlockIf -and (Is-Running @($it.BlockIf))) { Write-Host ("     -> 跳过（请先退出 " + ($it.BlockIf -join '/') + " 后重跑）") -ForegroundColor Yellow; continue }

    # 确认
    $do = $autoYes
    if (-not $autoYes) {
        $ans = "$(& { Read-Host '     删除? [y/N/a/q]' })".Trim().ToLower()
        if ($ans -eq 'q') { Write-Host "已退出。" -ForegroundColor Yellow; break }
        if ($ans -eq 'a') { $autoYes = $true; $do = $true }
        elseif ($ans -eq 'y') { $do = $true } else { $do = $false }
    }
    if (-not $do) { Write-Host "     -> 跳过" -ForegroundColor DarkGray; continue }

    # 执行
    try {
        switch ($it.Kind) {
            'hibernate' {
                Write-Host "     执行 powercfg /h off ..." -ForegroundColor DarkGray
                powercfg /h off
                Write-Host "     完成" -ForegroundColor Green
                break
            }
            'dism' {
                Write-Host "     执行 dism /online /cleanup-image /startcomponentcleanup（耐心等）..." -ForegroundColor DarkGray
                dism /online /cleanup-image /startcomponentcleanup
                Write-Host "     完成" -ForegroundColor Green
                break
            }
            'npm' {
                Write-Host "     执行 npm cache clean --force ..." -ForegroundColor DarkGray
                npm cache clean --force
                Write-Host "     完成" -ForegroundColor Green
                break
            }
            'delete' {
                foreach ($p in $it.Paths) { if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Recurse -Force -EA SilentlyContinue } }
                break
            }
            'delete-children' {
                $p = $it.Paths[0]
                if (Test-Path -LiteralPath $p) { Get-ChildItem -LiteralPath $p -Force -EA SilentlyContinue | Remove-Item -Recurse -Force -EA SilentlyContinue }
                break
            }
            'updater-glob' {
                Get-ChildItem -LiteralPath $env:LOCALAPPDATA -Directory -Filter '*updater*' -Force -EA SilentlyContinue | ForEach-Object {
                    Remove-Item -LiteralPath $_.FullName -Recurse -Force -EA SilentlyContinue
                }
                break
            }
        }
        # 释放量
        $sizeAfter = Calc-Item $it
        if ($sizeBefore -is [double]) {
            $afterVal = if ($sizeAfter -is [double]) { $sizeAfter } else { 0 }
            $freed = [math]::Round($sizeBefore - $afterVal, 2)
            if ($freed -lt 0) { $freed = 0 }
            $totalFreed += $freed
            $extra = if ($afterVal -ge 0.01) { "  (余 " + (fmt $afterVal) + " 被占用未删)" } else { '' }
            Write-Host ("     完成，释放 ~" + (fmt $freed) + $extra) -ForegroundColor Green
        }
    } catch {
        Write-Host ("     出错: " + $_.Exception.Message) -ForegroundColor Red
    }
}

# ---- 总结 ----
$after = Get-FreeGB
Write-Host ""
Write-Host "=== 完成 ===" -ForegroundColor Cyan
Write-Host ("本次累计释放估算: " + (fmt $totalFreed)) -ForegroundColor Green
Write-Host ("C 盘剩余: $before GB  ->  $after GB   (实际 +" + [math]::Round($after - $before, 1) + " GB)")

# ---- 应用数据提示（不自动删，避免误删聊天/文件）----
Write-Host ""
Write-Host "以下为聊天/应用数据，脚本未自动删，建议在应用内清理（可改存储路径到 D 盘）:" -ForegroundColor Yellow
foreach ($r in @(
    @('企业微信 WXWork', "$env:APPDATA\Tencent\WXWork"),
    @('微信 WeChat',     "$env:APPDATA\Tencent\WeChat"),
    @('腾讯会议 WeMeet', "$env:APPDATA\Tencent\WeMeet"),
    @('Chrome Profile 1(废弃账号?)', "$env:LOCALAPPDATA\Google\Chrome\User Data\Profile 1")
)) {
    $g = Get-SizeGB $r[1]
    Write-Host ("  - {0,-26} {1}" -f $r[0], (fmt $g))
}
Write-Host ""
