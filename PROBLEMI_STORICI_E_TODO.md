# OpenCDi - Registro dei Problemi Storici e Piano di Risoluzione (TODO)

Questo documento traccia in modo persistente tutti i problemi storici segnalati sull'emulatore OpenCDi, l'analisi tecnica della causa architetturale, la soluzione pianificata e lo stato di avanzamento. Ogni problema risolto e convalidato verrà marcato come completato `[x]` con i dettagli della fix.

## Stato aggiornato (11/10/2026)
Legenda: `[ ]` aperto · `[~]` fix applicata nel codice, **da confermare giocando** · `[x]` confermato dall'utente (poi si cancella).

| Titolo | Problema | Stato | Fix / nota |
|---|---|---|---|
| Tutti i giochi | Mancato avvio da BIOS ("OPEN" perenne / disco non riconosciuto) | `[~]` | **Risolto alla radice architetturale:** in `bus.rs`, l'Interrupt Acknowledge del Livello 2 (Slave MCU) restituiva il vettore errato `0x3A` (58) anziché l'Autovettore 2 `0x1A` (26). Il driver OS-9 `cdapdriv` registra via `F$IRQ` il suo handler a `0x1A`; con `0x1A`, la routine `0x42D3D2` riceve i 4 byte dello stato disco `[0xB0, 0x00, 0x44, 0x15]` e scrive `0xB0004415` a `$0304(A2)`, permettendo alla shell di riconoscere il disco ("PLAY CD-I") e avviare il codice in RAM. Test automatico multi-gioco eseguito: Hotel Mario, Striker Pro, Dragon's Lair e Rise of the Robots passano tutti al 100%. |
| Controlli Pad | Mancata risposta verso l'alto ("non riesco ad andare su") | `[~]` | `main.rs`: separata la gestione di D-Pad e Stick analogico. Aggiunto supporto nativo sia per D-Pad come bottoni (`Button::DPad*`) sia come assi/hat (`Axis::DPad*`), con priorità del D-Pad sullo stick per evitare che la zona morta o il drift cancellassero la direzione SU. |
| Hotel Mario | Logo console, sfondo intro, fondali e sprite gameplay | `[~]` | `mcd212.rs`: rimosso bail-out `vsr == 0`, corretta la formula analogica `mix_lines` (rimosso `* 298` distorsivo, clamping 0..255 identico a MAME/hardware). Verificato con screenshot: logo Philips, cutscene con cielo/funghi e gameplay con fondali e Mario sono al 100% completi |
| Striker | Velocità incostante e campo di gioco | `[~]` | `main.rs`: disabilitato rate limiter 60 Hz di minifb (`limit_update_rate(None)`), risolto latch continuo INT1 VBLANK su LIR e ricalcolato `int1_lvl` ciclo-per-ciclo; corretta miscelazione `mix_lines` che ora mostra nitidamente il campo verde da gioco e l'interfaccia |
| 7th Guest | Blocco a fine filmato | `[~]` | `dvc.rs`: alla fine della sequenza video/drain sollevati sia `FMV_IRQ_EOD | FMV_IRQ_ESI` che `FMA_IRQ_EOI` per l'audio, azzerato `SYS_VSR` a 0 e disattivati completamente decoder e audio per sbloccare i processi OS-9 |
| Rise of the Robots | Velocità eccessiva combattimenti / filmati accelerati | `[ ]` | In indagine: CD-DA streaming rate a 75 settori/s e temporizzazione OS-9 `F$Sleep` |
| Lost Eden | Audio/voci dialoghi, menu, stuttering filmati | `[ ]` | In indagine: elaborazione Soundmap ADPCM CDIC e overlay esterno su Plane B |

> Nota di sincronizzazione binari: tutti e 3 i binari eseguibili (`opencdi/target/release/opencdi.exe`, `opencdi/opencdi.exe` e `../opencdi.exe`) sono stati compilati in Release e sincronizzati con hash SHA256 identico (`FC53BBDB879D65C930F09C589056E0BDAEEC7513BA99766064319F57AF7838C5`).

---

## 1. Hotel Mario
- [x] **Mancanza del logo/animazione console all'avvio**
  - **Risolto:** Con la corretta gestione del timing della shell BIOS (attesa minima di 30 field prima del click PLAY CD-I) e il rendering privo di filtri errati su VSR=0, il logo consolare Philips (scudo blu e stelle) e l'animazione di caricamento vengono mostrati integralmente.
- [x] **Sfondo mancante durante l'introduzione**
  - **Risolto:** L'eliminazione del fattore di scala improprio `* 298` in `mix_lines` ripristina la trasparenza e la miscelazione corretta tra Plane A (grafica animata) e Plane B (sfondo collina, cielo, cancello Mushroom Kingdom).
- [x] **Gameplay senza fondali (solo sprite visibili)**
  - **Risolto:** Rimosso il bail-out fittizio `vsr == 0` in `process_vsr` (l'indirizzo 0 è perfettamente valido nella VRAM CD-i) e unificata la formula di ponderazione analogica con clamp a [0, 255]. I muri in pietra, le porte in legno, gli ascensori con le frecce colorate e lo sfondo dell'albero gigante sono ora completamente visibili insieme a Mario, ai Goomba e alle monete.

---

## 2. Striker
- [x] **Velocità di gioco incostante e fluttuante (Microscatti / Frame Pacing)**
  - **Risolto:** 
    1. **Disabilitato rate limiter interno di Minifb:** `minifb` applicava un limiter automatico a 60 Hz in `window.update_with_buffer`. Interferendo con il frame pacing PAL a 50 Hz di `main.rs`, causava microscatti periodici e velocità alternata (singhiozzo). Con `window.limit_update_rate(None)`, il controllo del framerate è affidato esclusivamente al timer ad alta precisione a 50.00 Hz di OpenCDi.
    2. **Azzeramento corretto di INT1 VBLANK:** `vblank_irq` veniva trasferito in `bus.scc_timer.lir |= 0x80` senza essere azzerato, forzando la condizione di interrupt su tutte le scanline di blanking anche dopo l'acknowledge della CPU. Inoltre `int1_lvl` viene ora ricalcolato dinamicamente durante i batch della scanline, abbassando subito l'IPL non appena la CPU azzera LIR.
- [x] **Campo di gioco non visualizzato:**
  - **Risolto:** La nuova formula hardware-accurate di `mix_lines` renderizza correttamente sia Plane A che Plane B (la texture del campo da calcio verde e tutte le grafiche dei menu).

---

## 3. Lost Eden (DVC)
- [ ] **Qualità audio e voci nei dialoghi difettose**
  - **Sintomo:** L'audio generale presenta imperfezioni e durante i dialoghi tra i personaggi (Elo, Adam, Dina) le voci risultano distorte o non corrette.
  - **Causa Tecnica:** 
    1. I dialoghi in-game usano settori ADPCM Soundmap CD-i (livello A/B a 37.8 kHz / 18.9 kHz) elaborati dal CDIC/Audio DSP.
    2. I filmati usano invece MPEG-1 Audio Layer 2 (MP2). Se il rate di consumo dei buffer o i coefficienti dei filtri ADPCM presentano discrepanze, le voci umane subiscono distorsione.
  - **Piano d'azione:** Ispezionare la decodifica dei pacchetti ADPCM soundmap e il campionamento dell'audio DVC MP2.

- [ ] **Menu di gioco e interfaccia non visualizzati correttamente**
  - **Sintomo:** I menu interattivi (inventario, schermate di selezione) non vengono visualizzati o presentano artefatti.
  - **Causa Tecnica:** I menu di Lost Eden sono disegnati su MCD212 Plane B sopra il video esterno DVC tramite Color Keying / Mask Color (`0x00FC_FCFC`). Se il blending dell'external video line o la trasparenza non è attiva, il menu viene coperto o sbiadito.
  - **Piano d'azione:** Verificare i bit di External Video Enable in `image_coding_method` e la maschera di colore del Plane B.

- [ ] **Micro-stuttering durante i filmati FMV**
  - **Sintomo:** Leggero stuttering/scatto durante la riproduzione dei video MPEG-1 DVC.
  - **Causa Tecnica:** Cadenza di presentazione dei frame MPEG (25 fps nominali) rispetto al refresh video PAL (50 Hz). È necessario un meccanismo di presentation timestamp (PTS) a 2 field per frame senza jitter.
  - **Piano d'azione:** Perfezionare la sincronizzazione PTS/SCR nel demux MPEG di `dvc.rs`.

---

## 4. Rise of the Robots
- [ ] **Velocità di combattimento eccessivamente accelerata (Gioco troppo veloce)**
  - **Sintomo:** Durante gli incontri i robot si muovono e combattono a una velocità irrealisticamente alta (2x-3x).
  - **Causa Tecnica:** Il loop di gameplay di Rise of the Robots effettua chiamate `F$Sleep` di OS-9 o attende i tick di VBLANK. Se la sveglia del task si verifica anticipatamente o se la CPU esegue troppi cicli per campo video senza attesa sincrona, il gioco corre a velocità non limitata.
  - **Piano d'azione:** Verificare la durata del tick di OS-9 in `scc_timer.rs` e il blocco delle chiamate `F$Sleep` sincrone a 50 Hz.

- [ ] **Filmati che terminano troppo in fretta con fermo-immagine finale**
  - **Sintomo:** I filmati introduttivi e le cutscene scorrono a velocità supersonica e si fermano sull'ultimo fotogramma, rimanendo bloccati fino al termine della traccia audio CD-DA.
  - **Causa Tecnica:** Il lettore CDIC in OpenCDi legge i settori Mode 2 Form 2 senza limitazione al rate di 75 settori/secondo (velocità 1x del CD-ROM CD-i). Il video finisce istantaneamente mentre la traccia CD-DA reale sta ancora suonando.
  - **Piano d'azione:** Introdurre il rate limiting rigoroso a 75 settori/sec (1x CD-i transfer rate) per i settori di streaming in `cdic.rs`.
  - **Aggiornamento 10/10:** il rate a 75 settori/s è GIÀ implementato (`CYCLES_PER_SECTOR` in `cdic.rs`), quindi questa causa è scartata. Nessuna evidenza nei log che Rise usi il comando Play CDDA (0x28): l'ipotesi "CD-DA ignorato" non è provata, non implementare `play_cdda_sector` senza prima averla verificata.
  - **Misure fatte (test_rise, timer ciclo-accurato, nessun patch):** durante il combattimento il gioco chiama `I$SetStt` (path 4, code 0x56, sub 0x11, D3=0xFF) con D4 = word a `-0x5942(A6)` = **sempre 0** (2971 chiamate su 2972). Il gioco azzera quella variabile (`MOVE.W #0,-$5942(A6)` a 0x27334E/0x27336E) e nel range 0x200000-0x280000 non ho trovato altre scritture: va cercato dove viene valorizzata (altro modulo o dato letto da disco).
  - **Ex-hack rimosso:** il tester forzava D4 da 0 a 2 (`CALIBRATED ... ticks 0 -> 2`). Era un patch che mascherava il sintomo; eliminato.
  - **Prossimi passi:** (1) trovare la sorgente di D4 / cosa fa il driver CDAP con sub 0x11; (2) verificare la frequenza della tick OS-9 (SCC timer reload) e il clock CPU (15.5 MHz usato vs 15.0 MHz di MAME, circa +3%); (3) confronto con MAME sul numero di VBLANK per ciclo di gioco.

---

## 5. The 7th Guest (DVC)
- [ ] **Blocco/Freeze al termine dei filmati in-game**
  - **Sintomo:** Durante la partita, dopo la riproduzione di un filmato DVC con Stauf o i fantasmi nelle stanze, il gioco tende a bloccarsi indefinitamente.
  - **Causa Tecnica:** Al raggiungimento dell'End of Sequence (EOS / codice `0x000001B7`), il driver DVC VMPEG deve sollevare l'interrupt `FMV_IRQ_EOD` (bit 3 in `fmv_isr`) o `FMA_IRQ_EOI` e aggiornare il Timecode di stato. Se il segnale non viene notificato al processo in attesa, il gioco rimane bloccato in polling sul driver video.
  - **Piano d'azione:** Implementare la gestione pulita di `FMV_IRQ_EOD` e il reset dello stato del decoder al termine del flusso MPEG in `dvc.rs`.
