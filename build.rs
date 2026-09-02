// Derleme anı bilgisi: git commit'i ve tarih, !durum ve açılış duyurusu için.
// Dış kütüphane yok; git ya da date yoksa "?" kalır, derleme durmaz.
use std::process::Command;

fn komut(program: &str, args: &[&str]) -> Option<String> {
    let cikti = Command::new(program).args(args).output().ok()?;
    if !cikti.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&cikti.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn main() {
    let mut commit = komut("git", &["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "?".into());
    // çalışma ağacında commit'lenmemiş değişiklik varsa "+" eki: hangi kodun koştuğu belli olsun
    if komut("git", &["status", "--porcelain"]).is_some() {
        commit.push('+');
    }
    let tarih = komut("date", &["+%Y-%m-%d"]).unwrap_or_else(|| "?".into());
    println!("cargo:rustc-env=SURUM_COMMIT={commit}");
    println!("cargo:rustc-env=SURUM_TARIH={tarih}");
    // commit değişince yeniden derlensin (HEAD ve o an bakılan dal)
    println!("cargo:rerun-if-changed=.git/HEAD");
    if let Some(dal) = komut("git", &["symbolic-ref", "-q", "HEAD"]) {
        println!("cargo:rerun-if-changed=.git/{dal}");
    }
}
