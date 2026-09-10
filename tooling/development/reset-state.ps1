$ErrorActionPreference = 'Stop'

$running = Get-Process -Name 'nanika-desktop' -ErrorAction SilentlyContinue
if ($null -ne $running) {
    throw 'Stop the running Nanika development application before resetting its state.'
}

if ([string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
    throw 'LOCALAPPDATA must be set.'
}

$localData = [System.IO.Path]::GetFullPath($env:LOCALAPPDATA)
$productData = [System.IO.Path]::GetFullPath(
    [System.IO.Path]::Combine($localData, 'nanika', 'nanika', 'data')
)
$webviewData = [System.IO.Path]::GetFullPath(
    [System.IO.Path]::Combine($localData, 'com.nanika.nanika')
)
$expectedProductData = [System.IO.Path]::Combine($localData, 'nanika', 'nanika', 'data')
$expectedWebviewData = [System.IO.Path]::Combine($localData, 'com.nanika.nanika')

if ($productData -ne $expectedProductData -or $webviewData -ne $expectedWebviewData) {
    throw 'Refusing to remove unexpected development paths.'
}

Remove-Item -LiteralPath $productData, $webviewData -Recurse -Force -ErrorAction SilentlyContinue
