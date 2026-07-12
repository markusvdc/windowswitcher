$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot

function Invoke-Check {
	param(
		[Parameter(Mandatory = $true)]
		[string]$Name,

		[Parameter(Mandatory = $true)]
		[scriptblock]$Command
	)

	Write-Host ""
	Write-Host "==> $Name" -ForegroundColor Cyan
	& $Command

	if ($LASTEXITCODE -ne 0) {
		Write-Error "$Name falhou com exit code $LASTEXITCODE."
		exit $LASTEXITCODE
	}

	Write-Host "OK: $Name" -ForegroundColor Green
}

Push-Location $repositoryRoot

try {
	Write-Host "Verificando o projeto em: $repositoryRoot"

	Invoke-Check "Formatacao (rustfmt)" {
		cargo fmt --all --check
	}

	Invoke-Check "Analise estatica (Clippy)" {
		cargo clippy --workspace --all-targets --all-features -- -D warnings
	}

	Invoke-Check "Testes" {
		cargo test --workspace --all-targets --all-features
	}

	Write-Host ""
	Write-Host "Todas as verificacoes passaram." -ForegroundColor Green
} finally {
	Pop-Location
}
