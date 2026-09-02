impl Bot {
    // !uyan ve ayar paneli: planı silme (silinirse dakika sonra yeniden kurulup tekrar
    // uyutur), planlı uyku bitene kadar "zorla uyanık" kal
    pub fn uyandir(&self) {
        let mut d = self.durum();
        let simdi = simdi_unix();
        let bitis = d
            .planlar
            .iter()
            .filter(|p| p.bas <= simdi && simdi < p.bit)
            .map(|p| p.bit)
            .max()
            .unwrap_or(simdi + 6 * 3600);
        d.uyanik_zorla = bitis;
    }

    // !uyu [saat] ve ayar paneli: test için geçici uyku planı
    pub fn uyut(&self, saat: i64) {
        let mut d = self.durum();
        let simdi = simdi_unix();
        d.uyanik_zorla = 0;
        d.planlar.push(uyku::Plan {
            gun: -1,
            uykusuz_bas: None,
            bas: simdi,
            bit: simdi + saat * 3600,
        });
    }

    // !debug aç|kapat (boşsa tersine çevirir); durum/debug.md'de kalıcı. Yeni durumu döner
    pub fn debug_ayarla(&self, arg: &str) -> bool {
        let mut d = self.durum();
        let yeni = match arg.trim().to_lowercase().as_str() {
            "aç" | "ac" | "açık" | "acik" | "on" => true,
            "kapat" | "kapalı" | "kapali" | "off" => false,
            _ => !d.debug,
        };
        d.debug = yeni;
        hafiza::yaz("debug.md", if yeni { "acik" } else { "kapali" });
        yeni
    }

    // openrouter model listesinde var mı
    async fn model_var_mi(&self, id: &str) -> bool {
        #[derive(Deserialize)]
        struct Liste {
            data: Vec<Kayit>,
        }
        #[derive(Deserialize)]
        struct Kayit {
            id: String,
        }
        match self
            .http
            .get("https://openrouter.ai/api/v1/models")
            .send()
            .await
        {
            Ok(r) => r
                .json::<Liste>()
                .await
                .map(|l| l.data.iter().any(|k| k.id == id))
                .unwrap_or(false),
            Err(_) => true, // liste çekilemediyse engel olma
        }
    }
}

