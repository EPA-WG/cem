export function renderFormula({ archiveSha256, archiveUrl, version }) {
    return `class CemMl < Formula
  desc "Schema-defined CEM parser, validator, query, and transformation CLI"
  homepage "https://github.com/EPA-WG/cem"
  url "${archiveUrl}"
  sha256 "${archiveSha256}"
  license "MIT"
  version "${version}"

  def install
    bin.install "bin/cem-ml"
    pkgshare.install "share/cem-ml/capabilities.json", "share/cem-ml/build-metadata.json"
  end

  test do
    (testpath/"smoke.cem").write <<~CEM
      @doc cem-ml 1
      @ns html = "http://www.w3.org/1999/xhtml"
      @default html

      {main | {h1 | Homebrew smoke}}
    CEM
    validation = shell_output("#{bin}/cem-ml validate #{testpath}/smoke.cem --format json")
    assert_match %q("hardViolationCount": 0), validation
    conversion = shell_output("#{bin}/cem-ml convert #{testpath}/smoke.cem --to-format dom-json")
    assert_match %q("kind": "document"), conversion
  end
end
`;
}
