# dev/ — oturum hafızası

Bu klasör botun geliştirme sürecinin **tek güvenilir hafızasıdır**.

- Context compact edilirse ya da yeni bir ajan oturumu başlarsa: önce `AGENTS.md`,
  sonra **bu klasör** (`ilerleme.md` → `yol-haritasi.md`) okunur.
- Her anlamlı adımda (commit ölçeğinde) `ilerleme.md`'ye kronolojik not düşülür.
- Plan değişirse `yol-haritasi.md` güncellenir; eski plan silinmez, üstü çizilir/yanına yazılır.
- Buraya yazılanlar **genel ve kalıcı** olmalı: anlık debug çıktıları, token, ortam
  adresi gibi şeyler yazılmaz (onlar `.env`'de ve git dışı).

## Dosyalar
| Dosya | İçerik |
|---|---|
| `ilerleme.md` | Yapılanların kronolojisi: tarih, commit, ne+neden, doğrulama durumu |
| `yol-haritasi.md` | Açık plan: sıradaki adımlar, öncelik, bağımlılık, bilinen riskler |
