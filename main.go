package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"math/rand"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"sync"
	"time"

	"github.com/bwmarrin/discordgo"
	"github.com/joho/godotenv"
	openai "github.com/sashabaranov/go-openai"
)

const (
	model         = "openai/gpt-4o-mini"
	maxMesaj      = 12            // bir sohbette en fazla kaç mesaj yazar
	vedaEsigi     = 9             // bu sayıdan sonra konuyu kapatmaya çalışır
	sans          = 0.1           // normal mesajlaşmaya %10 ihtimalle dalar
	bekleme       = 3 * time.Hour // sohbetten kaçınca 3 saat o kanala girmez
	yorumSuresi   = 2 * time.Hour // haber attıktan sonra 2 saat yorum bekler
	haberAraligi  = 6 * time.Hour // ne sıklıkla hacker news'e bakar
	gecmisGun     = 14            // açılışta kaç günlük mesaj okur
	hafizaBoyu    = 2000          // akılda tuttuğu son mesaj sayısı
	dürtmeAraligi = 1 * time.Hour // ne sıklıkla kendiliğinden laf atmayı dener
	dürtmeSansi   = 0.3           // her denemede %30 ihtimalle atar

)

var (
	ai          *openai.Client
	haberKanali string

	mu            sync.Mutex
	sohbetler     = map[string]*sohbet{}   // kanal id -> açık sohbet
	yasakli       = map[string]time.Time{} // kanal id -> tekrar girebileceği zaman
	haberBekleyen = map[string]time.Time{} // kanal id -> yorum beklemenin bittiği zaman

	botAdi   string   // discord'daki kullanıcı adı, kişilik metnine giriyor
	hafiza   []string // sunucuda okuduğu son mesajlar, "isim: metin" halinde
	profil   string   // hafızadan çıkardığı grup özeti
	sonKanal string   // en son konuşulan kanal, dürtme buraya gider
)

type sohbet struct {
	gecmis []openai.ChatCompletionMessage
	sayac  int
}

// ---------- yapay zeka ----------

// kişilikle konuşur: sohbet, hoş geldin, laf atma, haber tanıtma
func uret(gecmis []openai.ChatCompletionMessage, ekTalimat string) (string, error) {
	sistem := fmt.Sprintf(kisilik, botAdi)
	if profil != "" {
		sistem += "\n\nBU GRUP HAKKINDA BİLDİKLERİN\n" + profil
	}
	if ekTalimat != "" {
		sistem += "\n\nŞU ANKİ GÖREVİN\n" + ekTalimat
	}
	return sor(sistem, gecmis)
}

// kişiliksiz, düz analiz: profil çıkarma, haber seçme
func analiz(metin, talimat string) (string, error) {
	return sor(analist, []openai.ChatCompletionMessage{
		{Role: openai.ChatMessageRoleUser, Content: metin + "\n\n---\n\n" + talimat},
	})
}

func sor(sistem string, gecmis []openai.ChatCompletionMessage) (string, error) {
	mesajlar := append([]openai.ChatCompletionMessage{
		{Role: openai.ChatMessageRoleSystem, Content: sistem},
	}, gecmis...)

	cevap, err := ai.CreateChatCompletion(context.Background(), openai.ChatCompletionRequest{
		Model:    model,
		Messages: mesajlar,
	})
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(cevap.Choices[0].Message.Content), nil
}

// ---------- sohbet mekanizması ----------

func sohbetBaslat(kanalID, acilis string) {
	s := &sohbet{}
	if acilis != "" {
		s.gecmis = append(s.gecmis, openai.ChatCompletionMessage{Role: openai.ChatMessageRoleAssistant, Content: acilis})
		s.sayac = 1
	}
	sohbetler[kanalID] = s
}

func sohbetBitir(kanalID string) {
	delete(sohbetler, kanalID)
	delete(haberBekleyen, kanalID)
	yasakli[kanalID] = time.Now().Add(bekleme)
}

func girebilirMi(kanalID string) bool {
	return time.Now().After(yasakli[kanalID])
}

func sohbetDevam(dc *discordgo.Session, kanalID, kullanici, metin string) {
	mu.Lock()
	s, var_ := sohbetler[kanalID]
	if !var_ {
		mu.Unlock()
		return
	}
	s.gecmis = append(s.gecmis, openai.ChatCompletionMessage{Role: openai.ChatMessageRoleUser, Content: kullanici + ": " + metin})
	if len(s.gecmis) > 20 {
		s.gecmis = s.gecmis[len(s.gecmis)-20:]
	}

	talimat := ""
	switch {
	case s.sayac >= maxMesaj-1:
		talimat = sonMesaj
	case s.sayac >= vedaEsigi:
		talimat = vedaYaklasiyor
	}
	gecmis := append([]openai.ChatCompletionMessage{}, s.gecmis...)
	mu.Unlock()

	cevap, err := uret(gecmis, talimat)
	if err != nil {
		log.Println("ai hatası:", err)
		return
	}
	dc.ChannelMessageSend(kanalID, cevap)

	mu.Lock()
	defer mu.Unlock()
	s.gecmis = append(s.gecmis, openai.ChatCompletionMessage{Role: openai.ChatMessageRoleAssistant, Content: cevap})
	s.sayac++
	if s.sayac >= maxMesaj {
		sohbetBitir(kanalID)
	}
}

// ---------- hafıza ----------

func hatirla(isim, metin string) {
	hafiza = append(hafiza, isim+": "+metin)
	if len(hafiza) > hafizaBoyu {
		hafiza = hafiza[len(hafiza)-hafizaBoyu:]
	}
}

func sonMesajlar(n int) string {
	if len(hafiza) < n {
		n = len(hafiza)
	}
	return strings.Join(hafiza[len(hafiza)-n:], "\n")
}

// açılışta sunucudaki kanalların son iki haftasını okur
func gecmisiOku(dc *discordgo.Session, g *discordgo.Guild) {
	sinir := time.Now().AddDate(0, 0, -gecmisGun)
	var toplam []*discordgo.Message

	for _, k := range g.Channels {
		if k.Type != discordgo.ChannelTypeGuildText {
			continue
		}
		oncesi := ""
		for {
			parca, err := dc.ChannelMessages(k.ID, 100, oncesi, "", "")
			if err != nil || len(parca) == 0 {
				break
			}
			for _, m := range parca {
				if m.Timestamp.Before(sinir) {
					break
				}
				if !m.Author.Bot && m.Content != "" {
					toplam = append(toplam, m)
				}
			}
			son := parca[len(parca)-1]
			if son.Timestamp.Before(sinir) {
				break
			}
			oncesi = son.ID
		}
	}

	// discord yeniden eskiye verir, biz eskiden yeniye istiyoruz
	mu.Lock()
	for i := len(toplam) - 1; i >= 0; i-- {
		hatirla(toplam[i].Author.Username, toplam[i].Content)
	}
	mu.Unlock()
	log.Printf("%s: %d mesaj okundu", g.Name, len(toplam))
}

func profilGuncelle() {
	mu.Lock()
	ornek := sonMesajlar(600)
	mu.Unlock()
	if ornek == "" {
		return
	}

	yeni, err := analiz(ornek, profilCikar)
	if err != nil {
		log.Println("profil hatası:", err)
		return
	}

	mu.Lock()
	profil = yeni
	mu.Unlock()
	os.WriteFile("profil.txt", []byte(yeni), 0644)
	log.Println("profil güncellendi")
}

// arada bir, tanıdık biri gibi durup dururken laf atar
func dürt(dc *discordgo.Session) {
	for {
		time.Sleep(dürtmeAraligi)
		if rand.Float64() > dürtmeSansi {
			continue
		}

		mu.Lock()
		kanalID := sonKanal
		_, acik := sohbetler[kanalID]
		uygun := kanalID != "" && !acik && girebilirMi(kanalID) && profil != ""
		son := sonMesajlar(40)
		mu.Unlock()
		if !uygun {
			continue
		}

		laf, err := uret(
			[]openai.ChatCompletionMessage{{Role: openai.ChatMessageRoleUser, Content: son}},
			durupDururken,
		)
		if err != nil {
			log.Println("ai hatası:", err)
			continue
		}
		dc.ChannelMessageSend(kanalID, laf)

		mu.Lock()
		sohbetBaslat(kanalID, laf)
		mu.Unlock()
	}
}

// ---------- hacker news ----------

type haber struct {
	ID    int    `json:"id"`
	Title string `json:"title"`
	URL   string `json:"url"`
	Score int    `json:"score"`
}

func al(url string, hedef any) error {
	r, err := http.Get(url)
	if err != nil {
		return err
	}
	defer r.Body.Close()
	return json.NewDecoder(r.Body).Decode(hedef)
}

func hnHaber() (haber, error) {
	var idler []int
	if err := al("https://hacker-news.firebaseio.com/v0/topstories.json", &idler); err != nil {
		return haber{}, err
	}
	if len(idler) > 15 {
		idler = idler[:15]
	}

	var haberler []haber
	var liste strings.Builder
	for _, id := range idler {
		var h haber
		if err := al(fmt.Sprintf("https://hacker-news.firebaseio.com/v0/item/%d.json", id), &h); err != nil || h.Title == "" {
			continue
		}
		fmt.Fprintf(&liste, "%d. %s (%d puan)\n", len(haberler), h.Title, h.Score)
		haberler = append(haberler, h)
	}
	if len(haberler) == 0 {
		return haber{}, fmt.Errorf("haber bulunamadı")
	}

	mu.Lock()
	p := profil
	mu.Unlock()
	secim, err := analiz(liste.String(), fmt.Sprintf(haberSec, p))
	if err != nil {
		return haber{}, err
	}
	var n int
	if _, err := fmt.Sscanf(strings.TrimSpace(secim), "%d", &n); err != nil || n < 0 || n >= len(haberler) {
		n = 0
	}
	return haberler[n], nil
}

func haberKanaliBul(dc *discordgo.Session) string {
	for _, g := range dc.State.Guilds {
		for _, k := range g.Channels {
			if k.Type != discordgo.ChannelTypeGuildText {
				continue
			}
			if haberKanali != "" && k.ID == haberKanali {
				return k.ID
			}
			if haberKanali == "" {
				return k.ID
			}
		}
	}
	return ""
}

func haberPaylas(dc *discordgo.Session) {
	for {
		time.Sleep(haberAraligi)
		profilGuncelle()

		kanalID := haberKanaliBul(dc)
		mu.Lock()
		_, acik := sohbetler[kanalID]
		mu.Unlock()
		if kanalID == "" || acik {
			continue
		}

		h, err := hnHaber()
		if err != nil {
			log.Println("hn hatası:", err)
			continue
		}
		link := h.URL
		if link == "" {
			link = fmt.Sprintf("https://news.ycombinator.com/item?id=%d", h.ID)
		}
		girdi, err := uret(
			[]openai.ChatCompletionMessage{{Role: openai.ChatMessageRoleUser, Content: h.Title}},
			haberTanit,
		)
		if err != nil {
			log.Println("ai hatası:", err)
			continue
		}
		dc.ChannelMessageSend(kanalID, girdi+"\n"+link)

		mu.Lock()
		sohbetBaslat(kanalID, girdi)
		haberBekleyen[kanalID] = time.Now().Add(yorumSuresi)
		mu.Unlock()
	}
}

// ---------- olaylar ----------

func uyeKatildi(dc *discordgo.Session, e *discordgo.GuildMemberAdd) {
	g, err := dc.State.Guild(e.GuildID)
	if err != nil {
		return
	}
	kanalID := g.SystemChannelID
	if kanalID == "" {
		kanalID = haberKanaliBul(dc)
	}

	mu.Lock()
	uygun := kanalID != "" && girebilirMi(kanalID)
	mu.Unlock()
	if !uygun {
		return
	}

	isim := e.User.Username
	selam, err := uret(
		[]openai.ChatCompletionMessage{{Role: openai.ChatMessageRoleUser, Content: isim + " sunucuya yeni katıldı."}},
		hosGeldin,
	)
	if err != nil {
		log.Println("ai hatası:", err)
		return
	}
	dc.ChannelMessageSend(kanalID, e.User.Mention()+" "+selam)

	mu.Lock()
	sohbetBaslat(kanalID, selam)
	mu.Unlock()
}

func mesajGeldi(dc *discordgo.Session, m *discordgo.MessageCreate) {
	if m.Author.ID == dc.State.User.ID || m.GuildID == "" {
		return
	}
	kid := m.ChannelID

	mu.Lock()
	hatirla(m.Author.Username, m.Content)
	sonKanal = kid

	// haber attık, yorum bekliyoruz ama süre doldu
	if bitis, var_ := haberBekleyen[kid]; var_ && time.Now().After(bitis) {
		delete(sohbetler, kid)
		delete(haberBekleyen, kid)
	}

	_, acik := sohbetler[kid]
	if !acik && girebilirMi(kid) && rand.Float64() < sans {
		sohbetBaslat(kid, "")
		acik = true
	}
	mu.Unlock()

	if acik {
		go sohbetDevam(dc, kid, m.Author.Username, m.Content)
	}
}

func sunucuGeldi(dc *discordgo.Session, e *discordgo.GuildCreate) {
	go func() {
		gecmisiOku(dc, e.Guild)
		profilGuncelle()
	}()
}

// ---------- başlangıç ----------

func main() {
	godotenv.Load()
	haberKanali = os.Getenv("HABER_KANALI")

	cfg := openai.DefaultConfig(os.Getenv("OPENROUTER_KEY"))
	cfg.BaseURL = "https://openrouter.ai/api/v1" // openrouter, openai api'sinin aynısı
	ai = openai.NewClientWithConfig(cfg)

	dc, err := discordgo.New("Bot " + os.Getenv("DISCORD_TOKEN"))
	if err != nil {
		log.Fatal(err)
	}
	dc.Identify.Intents = discordgo.IntentsGuilds | discordgo.IntentsGuildMessages |
		discordgo.IntentsGuildMembers | discordgo.IntentMessageContent

	dc.AddHandler(mesajGeldi)
	dc.AddHandler(uyeKatildi)
	dc.AddHandler(sunucuGeldi)
	dc.AddHandler(func(s *discordgo.Session, r *discordgo.Ready) {
		botAdi = r.User.Username
		log.Println("giriş yapıldı:", botAdi)
	})

	if err := dc.Open(); err != nil {
		log.Fatal(err)
	}
	defer dc.Close()

	go haberPaylas(dc)
	go dürt(dc)

	// ctrl+c gelene kadar çalış
	dur := make(chan os.Signal, 1)
	signal.Notify(dur, os.Interrupt)
	<-dur
}
