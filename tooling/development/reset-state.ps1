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
    [System.IO.Path]::Combine($localData, 'Nanika')
)
# The embedded browser owns this state outside Nanika's application-managed roots.
$webviewState = [System.IO.Path]::GetFullPath(
    [System.IO.Path]::Combine($localData, 'app.nanika')
)
$expectedProductData = [System.IO.Path]::Combine($localData, 'Nanika')
$expectedWebviewState = [System.IO.Path]::Combine($localData, 'app.nanika')

if ($productData -ne $expectedProductData -or $webviewState -ne $expectedWebviewState) {
    throw 'Refusing to remove unexpected development paths.'
}

Remove-Item -LiteralPath $productData, $webviewState -Recurse -Force -ErrorAction SilentlyContinue
