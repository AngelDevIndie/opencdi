# OpenCDi - Registro dei Problemi Storici e Piano di Risoluzione (TODO)

[English Version](KNOWN_ISSUES_AND_TODO.md)

Questo documento traccia in modo persistente tutti i problemi storici segnalati sull'emulatore OpenCDi, l'analisi tecnica della causa architetturale, la soluzione pianificata e lo stato di avanzamento. Ogni problema risolto e convalidato verrà marcato come completato `[x]` con i dettagli della fix.

## Stato aggiornato (12/10/2026 - Versione v0.1.1)
Legenda: `[ ]` aperto · `[~]` fix applicata nel codice, **da confermare giocando** · `[x]` confermato dall'utente.

| Titolo | Problema | Stato | Fix / nota |
|---|---|---|---|
| Transizioni video (Hotel Mario, Striker) | Schermate intermedie con colori sbagliati/frammenti disordinati per pochi decimi di secondo | `[~]` | **Risolto alla radice architetturale (v0.1.1):** in `mcd212.rs`, `mix_lines` scavalcava la pesatura analogica quando un piano era trasparente (`out[x] = pa; continue;`), rivelando la VRAM non inizializzata a luminosità 100% durante i fade-out. Allineato a MAME e specifiche hardware: i piani trasparenti assumono il livello di nero `s_4bpp_color[0]` e passano per la formula con `weight_factor`. Con peso 0 l'uscita è ora nero solido autentico (verificato con `trace_video` su frame 2426–2468 di Hotel Mario). |
| Controlli Pad | Mancata risposta verso l'alto ("non riesco ad andare su") | `[~]` | **Risolto (v0.1.1):** in `main.rs`, disaccoppiate e gerarchizzate rigorosamente le direzioni pad: prima bottoni D-Pad (`Button::DPad*`), poi asse D-Pad (`Axis::DPad*`) solo se neutro, poi stick analogico (`LeftStickY`) solo se neutro. Eliminato il conflitto che sommava -1 e +1 azzerando il comando SU. |
| Striker | Flickering sprite dei giocatori e velocità scostante | `[ ]` | **In analisi attiva:** isolamento del ciclo di doppio buffer (VSR/DCP) e temporizzazione degli interrupt di riga/campo (DCA line 43 e ICA line 0). |
| Tutti i giochi | Mancato avvio da BIOS ("OPEN" perenne / disco non riconosciuto) | `[x]` | **Risolto:** in `bus.rs`, INTACK Livello 2 (Slave MCU) restituiva `0x3A` anziché Autovettore 2 `0x1A`. Con `0x1A`, OS-9 `cdapdriv` esegue `0x42D3D2` e riconosce lo stato disco ("PLAY CD-I"). |
| 7th Guest | Sfondo nero post-video risolto; blocco/freeze nel corridoio post-fantasma persiste | `[ ]` | **Sfondo nero risolto:** eliminato errato check `DCR_DE_BIT` su Path 1 in `mcd212.rs`. **Freeze corridoio APERTO:** indagine in corso sul meccanismo di cambio stanza / caricamento stream video successivo. |
| Rise of the Robots | Velocità eccessiva combattimenti / filmati accelerati | `[ ]` | In indagine: CD-DA streaming rate a 75 settori/s e temporizzazione OS-9 `F$Sleep` |
| Lost Eden | Audio/voci dialoghi, menu, stuttering filmati | `[ ]` | In indagine: elaborazione Soundmap ADPCM CDIC e overlay esterno su Plane B |

> Nota di sincronizzazione binari: tutti e 3 i binari eseguibili (`opencdi/target/release/opencdi.exe`, `opencdi/opencdi.exe` e `../opencdi.exe`) sono stati compilati in Release e sincronizzati con hash SHA256 identico:
> `99260CE0FE805FC5BFFECABC16433811BC5793F48BDCA2075B617AAF1B0169C3`.

---

## 1. Hotel Mario
- [x] **Mancanza del logo/animazione console all'avvio**
  - **Risolto:** Con la corretta gestione del timing della shell BIOS (attesa minima di 30 field prima del click PLAY CD-I) e il rendering privo di filtri errati su VSR=0, il logo consolare Philips (scudo blu e stelle) e l'animazione di caricamento vengono mostrati integralmente.
- [x] **Transizioni corrotte con colori psichedelici / disegno a vista**
  - **Risolto (v0.1.1):** In `mix_lines`, la trasparenza di Plane B non scavalca più la moltiplicazione per `weight_factor`. Durante il caricamento VRAM con `weight == 0`, lo schermo rimane nero pulito fino al termine della preparazione, eliminando qualsiasi artefatto.
- [x] **Sfondo mancante durante l'introduzione e Gameplay senza fondali**
  - **Risolto:** L'eliminazione del fattore di scala improprio `* 298` in `mix_lines` e la rimozione del bail-out `vsr == 0` ripristinano la trasparenza e la miscelazione corretta tra Plane A e Plane B.

---

## 2. Striker
- [ ] **Flickering sprite e velocità scostante durante la partita**
  - **In lavorazione:** Analisi delle cadenze di presentazione dei buffer e sincronizzazione dell'interrupt INT1.
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
- [x] **Schermata nera con elementi isolati/scontornati dopo le transizioni video**
  - **Sintomo:** Al termine delle transizioni filmate compare una schermata quasi completamente nera con solo alcuni elementi interattivi visibili (hotspot, oggetti isolati), senza lo sfondo della stanza.
  - **Causa Tecnica:** In `src/mcd212.rs`, era stato introdotto un controllo `(self.dcr[PATH] & DCR_DE_BIT) == 0` che forzava pixel neri e trasparenza totale. Nelle specifiche hardware Philips MCD212, solo DCR1 (Path 0) possiede il bit Display Enable globale; su DCR2 (Path 1 / Plane B) il bit 15 non è un plane-enable e non viene impostato dal gioco. Di conseguenza, Plane B (lo sfondo 3D della villa) veniva azzerato, lasciando solo gli overlay di Plane A su fondo nero.
  - **Risoluzione:** Rimosso il controllo errato in `process_vsr`. Sfondo 3D e oggetti interattivi tornano a essere visualizzati perfettamente fusi.

- [ ] **Blocco/Freeze al termine dei filmati in-game e post-fantasma nel corridoio**
  - **Sintomo:** Salendo le scale e girando a sinistra, dopo il filmato del fantasma nel corridoio il gioco si blocca tentando di avanzare.
  - **Stato:** `[ ]` In indagine attiva. Le migliorie all'arbitraggio IN4 e alla linea DMAREQ2 sono state integrate nell'architettura, ma non hanno sbloccato la transizione di gameplay successiva: il gioco continua a piantarsi nello stesso punto. Si stanno esaminando le chiamate I/O al driver CDIC e il caricamento del file video/modulo successivo dal disco.

*(Per la panoramica e lo stato di tutti i 15 giochi presenti nella cartella `file utili/giochi/`, consultare il file `COMPATIBILITY.md`)*

