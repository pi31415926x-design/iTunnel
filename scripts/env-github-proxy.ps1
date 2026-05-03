# Dot-source for current PowerShell session: . .\scripts\env-github-proxy.ps1
# Used by git/curl/gh when accessing GitHub through a local forward proxy.
$proxy = 'http://127.0.0.1:1080'
$env:HTTPS_PROXY = $proxy
$env:HTTP_PROXY = $proxy
$env:https_proxy = $proxy
$env:http_proxy = $proxy
Write-Host "HTTPS_PROXY / HTTP_PROXY = $proxy"
