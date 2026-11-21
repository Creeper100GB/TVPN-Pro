#requires -Version 5.1
<#
.SYNOPSIS
    Log redaction gate: fails when a log line contains data we never log.

.DESCRIPTION
    Scans log files for IPv4/IPv6 addresses (loopback excluded), MAC addresses
    and URLs. Findings are reported as "file:line: category" only. The matched
    text is never echoed, so this gate cannot spread a leak into CI output.

    Two modes:
      -Path <file...>   scan real logs, exit 1 on any finding
      -SelfTest         verify the detector against the fixtures in ./fixtures

    Exit codes: 0 clean, 1 findings or self-test failure, 2 usage error.

    This is a coarse net on purpose. It catches the three classes that are cheap
    and reliable to detect. Filenames, page titles and document names are NOT
    covered here; those are checked by rules in the scanner and by review.
#>
[CmdletBinding(DefaultParameterSetName = 'Scan')]
param(
    [Parameter(ParameterSetName = 'Scan', Position = 0)]
    [string[]]$Path,

    [Parameter(ParameterSetName = 'SelfTest', Mandatory = $true)]
    [switch]$SelfTest,

    [string]$FixtureDirectory
)

$ErrorActionPreference = 'Stop'

# $PSScriptRoot is not available in parameter defaults on Windows PowerShell 5.1.
if (-not $FixtureDirectory) {
    $FixtureDirectory = Join-Path $PSScriptRoot 'fixtures'
}

# IPv6 is validated with the platform parser instead of a monster regex:
# extract colon-separated hex candidates, then let IPAddress.TryParse decide.
$script:Ipv6Candidate = '[0-9A-Fa-f]{0,4}(?::[0-9A-Fa-f]{0,4}){2,7}'

$script:Patterns = @(
    @{
        Name  = 'ipv4'
        Regex = '(?<![\d.])(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}(?![\d.])'
    }
    @{
        Name  = 'mac'
        Regex = '(?:^|(?<=[^0-9A-Fa-f:-]))(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}(?=$|[^0-9A-Fa-f:-])'
    }
    @{
        Name  = 'url'
        Regex = '\b(?:https?|socks[45]?h?|ftp)://[^\s"''<>]+'
    }
)

function Test-AllowedIPv4 {
    param([string]$Text)

    $ip = $null
    if (-not [System.Net.IPAddress]::TryParse($Text, [ref]$ip)) {
        return $false
    }
    if ($ip.AddressFamily -ne [System.Net.Sockets.AddressFamily]::InterNetwork) {
        return $false
    }

    $bytes = $ip.GetAddressBytes()
    if ($bytes[0] -eq 127) { return $true }        # 127.0.0.0/8
    if ($Text -eq '0.0.0.0') { return $true }      # wildcard bind
    return $false
}

function Test-AllowedUrl {
    param([string]$Text)

    $uri = $null
    if (-not [System.Uri]::TryCreate($Text, [System.UriKind]::Absolute, [ref]$uri)) {
        return $false
    }
    return $uri.IsLoopback
}

function Get-LogFindings {
    param([string]$File)

    $findings = New-Object System.Collections.Generic.List[object]
    $lineNumber = 0

    foreach ($line in [System.IO.File]::ReadLines($File)) {
        $lineNumber++

        foreach ($pattern in $script:Patterns) {
            foreach ($match in [regex]::Matches($line, $pattern.Regex)) {
                $value = $match.Value
                if ($pattern.Name -eq 'ipv4' -and (Test-AllowedIPv4 $value)) { continue }
                if ($pattern.Name -eq 'url' -and (Test-AllowedUrl $value)) { continue }
                $findings.Add([pscustomobject]@{
                    File     = $File
                    Line     = $lineNumber
                    Category = $pattern.Name
                })
            }
        }

        foreach ($match in [regex]::Matches($line, $script:Ipv6Candidate)) {
            $ip = $null
            if (-not [System.Net.IPAddress]::TryParse($match.Value, [ref]$ip)) { continue }
            if ($ip.AddressFamily -ne [System.Net.Sockets.AddressFamily]::InterNetworkV6) { continue }
            if ([System.Net.IPAddress]::IsLoopback($ip)) { continue }
            if ([System.Net.IPAddress]::IPv6Any.Equals($ip)) { continue }
            $findings.Add([pscustomobject]@{
                File     = $File
                Line     = $lineNumber
                Category = 'ipv6'
            })
        }
    }

    return $findings
}

function Invoke-SelfTest {
    $leaky = Join-Path $FixtureDirectory 'leaky.log'
    $clean = Join-Path $FixtureDirectory 'clean.log'

    foreach ($fixture in @($leaky, $clean)) {
        if (-not (Test-Path -LiteralPath $fixture)) {
            throw "redaction selftest: fixture missing: $fixture"
        }
    }

    $leakyFindings = @(Get-LogFindings -File $leaky)
    $cleanFindings = @(Get-LogFindings -File $clean)

    $found = @($leakyFindings | Select-Object -ExpandProperty Category -Unique)
    $expected = @('ipv4', 'ipv6', 'mac', 'url')
    $missing = @($expected | Where-Object { $found -notcontains $_ })

    $ok = ($missing.Count -eq 0) -and ($cleanFindings.Count -eq 0)

    if (-not $ok) {
        foreach ($item in $leakyFindings) {
            Write-Host ('leaky fixture: {0}:{1}: {2}' -f (Split-Path -Leaf $item.File), $item.Line, $item.Category)
        }
        foreach ($item in $cleanFindings) {
            Write-Host ('clean fixture FALSE POSITIVE: {0}:{1}: {2}' -f (Split-Path -Leaf $item.File), $item.Line, $item.Category)
        }
        if ($missing.Count -gt 0) {
            Write-Host ('missing categories in leaky fixture: {0}' -f ($missing -join ', '))
        }
        Write-Host 'redaction selftest: FAIL'
        exit 1
    }

    Write-Host ('redaction selftest: PASS ({0} findings in leaky fixture, clean fixture clean)' -f $leakyFindings.Count)
    exit 0
}

if ($SelfTest) {
    Invoke-SelfTest
}

if (-not $Path -or $Path.Count -eq 0) {
    Write-Host 'usage: check-log-redaction.ps1 -Path <logfile...> | -SelfTest'
    exit 2
}

$total = 0
foreach ($file in $Path) {
    if (-not (Test-Path -LiteralPath $file)) {
        Write-Host "redaction: file not found: $file"
        exit 2
    }
    $resolved = (Resolve-Path -LiteralPath $file).Path
    $findings = @(Get-LogFindings -File $resolved)
    foreach ($finding in $findings) {
        Write-Host ('{0}:{1}: {2}' -f $finding.File, $finding.Line, $finding.Category)
    }
    $total += $findings.Count
}

if ($total -gt 0) {
    Write-Host ('redaction: {0} finding(s). Logs must not contain addresses, MACs or URLs.' -f $total)
    exit 1
}

Write-Host 'redaction: no findings'
exit 0
