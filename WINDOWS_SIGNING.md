# Windows Signing

CoworkPal release builds can sign Windows `.exe` and `.msi` bundles during the GitHub Actions release workflow.

## Optional GitHub Secrets

Add these repository secrets in `FiveDayZ/CoworkPal` when a trusted code signing certificate is available:

- `WINDOWS_CERTIFICATE`: Base64 encoded `.pfx` code signing certificate.
- `WINDOWS_CERTIFICATE_PASSWORD`: Export password for the `.pfx` certificate.
- `WINDOWS_SIGNING_TIMESTAMP_URL`: Optional timestamp server URL. If omitted, the workflow uses `http://timestamp.digicert.com`.

The release workflow keeps publishing unsigned Windows bundles when `WINDOWS_CERTIFICATE` or `WINDOWS_CERTIFICATE_PASSWORD` is missing. If both secrets are configured, the workflow attempts to sign the Windows bundles and fails on invalid certificate data or password errors.

## Convert a PFX to Base64

On Windows:

```powershell
certutil -encode certificate.pfx base64cert.txt
```

Copy the full contents of `base64cert.txt` into the `WINDOWS_CERTIFICATE` secret.

## Important

Self-signed certificates can prove that the file has not been modified, but they do not make Windows trust the publisher by default. To reduce SmartScreen warnings for public downloads, use a real code signing certificate from a trusted certificate authority. EV certificates generally gain SmartScreen reputation faster than OV certificates.
