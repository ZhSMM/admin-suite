// Strip YAML comments (lines starting with #) before scanning
const fs = require('fs');
const yaml = fs.readFileSync('.github/workflows/release.yml', 'utf8');
const stripped = yaml.split('\n').map(l => l.replace(/#.*$/, '')).join('\n');
// Only count ${{ and }} that look like GitHub Actions expressions:
//   - ${{ must be preceded by start-of-line, whitespace, or '=', and
//     followed by an identifier (not a digit, which would be a PowerShell
//     ${...} subexpression)
//   - }} must be preceded by anything that is not '{' (so we don't catch
//     double '}}' from PowerShell @{...} scriptblocks) and followed by
//     whitespace, end-of-line, '.', or ')'
const exprOpens = [...stripped.matchAll(/(^|\s|=)\$\{\{[^0-9]/gm)];
const exprCloses = [...stripped.matchAll(/[^}]\}\}(?=\s|$|\.|\))/gm)];
console.log('expr opens:', exprOpens.length, 'expr closes:', exprCloses.length);
if (exprOpens.length !== exprCloses.length) {
  console.log('MISMATCH');
  process.exit(1);
}
console.log('OK');
