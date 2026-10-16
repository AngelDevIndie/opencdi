# Lista di Compatibilità Giochi - OpenCDi

Questo documento traccia lo stato di compatibilità e i problemi riscontrati dei titoli CD-i testati con l'emulatore **OpenCDi**, con particolare attenzione alla gestione dell'hardware audio/video (MCD-212) e della cartuccia DVC (Digital Video Cartridge MPEG-1).

---

## Tabella Riassuntiva

| # | File / Titolo | DVC Richiesto | Stato Attuale | Note Principali |
|---|---|:---:|:---:|---|
| 1 | `Hotel Mario (USA).chd` | No | **Giocabile / Ottimo** | Audio CD-DA e ADPCM perfetti, grafica piani A/B corretta, controlli reattivi, cutscene stabili. |
| 2 | `Dragon's Lair (USA).chd` | **Sì** | **Giocabile / Ottimo** | Full-motion video MPEG a schermo intero fluido, attract mode e gameplay reattivo stabili, sincronizzazione CDIC corretta. |
| 3 | `Lost Eden (Europe).chd` | **Sì** | **Giocabile / Molto buono** | Video FMV e audio CD-DA/ADPCM sincronizzati, transizioni ambienti fluide. |
| 4 | `7th Guest, The (USA).chd` | **Sì** | **Giocabile / In perfezionamento** | Risolti i problemi di sfondo nero dopo le transizioni video e il blocco dopo il filmato del fantasma al piano superiore. |
| 5 | `Striker Pro (USA).chd` | No | **Avviabile / Giocabile** | Grafica 2D renderizzata su doppi piani, gameplay funzionante. |
| 6 | `Rise of the Robots (Europe).chd` | No | **Avviabile** | Intro e menù funzionanti; da verificare la stabilità in sessioni prolungate. |
| 7 | `alberoazzurro.chd` | No / Da verif. | *Non ancora testato* | Aggiunto alla cartella giochi, in attesa di test. |
| 8 | `Chaos Control (USA).chd` | **Sì** | *Non ancora testato* | Aggiunto alla cartella giochi, in attesa di test. |
| 9 | `Creature Shock (Europe) (Disc 1).chd` | **Sì** | *Non ancora testato* | Aggiunto alla cartella giochi, in attesa di test. |
| 10 | `Dark Fables of Aesop The (Europe).chd` | No / Da verif. | *Non ancora testato* | Aggiunto alla cartella giochi, in attesa di test. |
| 11 | `Escape from CyberCity (USA).chd` | **Sì** | *Non ancora testato* | Aggiunto alla cartella giochi, in attesa di test. |
| 12 | `International Tennis Open (Europe).chd` | No | *Non ancora testato* | Aggiunto alla cartella giochi, in attesa di test. |
| 13 | `Kingdom - The Far Reaches (USA).chd` | **Sì** | *Non ancora testato* | Aggiunto alla cartella giochi, in attesa di test. |
| 14 | `Tangram - The Ultimate Chinese Game (Europe).chd` | No | *Non ancora testato* | Aggiunto alla cartella giochi, in attesa di test. |
| 15 | `Ultra CD-i Soccer (Europe).chd` | No | *Non ancora testato* | Aggiunto alla cartella giochi, in attesa di test. |

---

## Dettaglio Titoli Testati

### 1. Hotel Mario (USA)
- **DVC:** Non necessario (grafica nativa MCD-212 Plane A/B).
- **Stato:** Pienamente giocabile.
- **Risoluzioni applicate:**
  - Corretta gestione delle palette CLUT e dei registri DCA/ICA per evitare sfarfallii.
  - Sincronizzazione interrupt Slave MCU (mouse/joypad) e CDIC ADPCM audio.

### 2. Dragon's Lair (USA)
- **DVC:** Richiesto (Digital Video Cartridge con MPEG-1 video e audio Layer 2).
- **Stato:** Pienamente giocabile.
- **Risoluzioni applicate:**
  - Evitato abort prematuro delle letture streaming CDIC sui cambi traccia/scena.
  - Temporizzazione DVC timer e gestione interrupt GOP/SEQ calibrata.

### 3. Lost Eden (Europe)
- **DVC:** Richiesto.
- **Stato:** Giocabile.
- **Risoluzioni applicate:**
  - Ripristinata la corretta rilevazione dell'End-of-Stream (EOS) sul buffer drain MPEG.
  - Timer FMV protetto da trigger spurii a decoder disattivo.

### 4. The 7th Guest (USA)
- **DVC:** Richiesto.
- **Stato:** Parzialmente giocabile / Freeze nel corridoio in indagine.
- **Problemi identificati:**
  1. **Schermata nera con elementi isolati a fine video:** Risolto. Rimosso errato controllo `DCR_DE_BIT` su Path 1 in `mcd212.rs`, ripristinando la visualizzazione corretta di sfondo 3D (Plane B) e hotspot interattivi (Plane A).
  2. **Freeze post-video fantasma al primo piano a sinistra:** In corso di analisi. Il gioco si blocca tentando di avanzare nel corridoio dopo la riproduzione del filmato; l'indagine è attiva sul flusso I/O e caricamento stream successivo.

### 5. Striker Pro (USA)
- **DVC:** Non necessario.
- **Stato:** Avviabile e giocabile.

### 6. Rise of the Robots (Europe)
- **DVC:** Non necessario.
- **Stato:** Avviabile (intro e menù).

---

## Titoli in Attesa di Test
I seguenti titoli sono stati censiti nella cartella `file utili/giochi/` e verranno collaudati sistematicamente:
- `alberoazzurro.chd`
- `Chaos Control (USA).chd` (DVC)
- `Creature Shock (Europe) (Disc 1).chd` (DVC)
- `Dark Fables of Aesop The (Europe).chd`
- `Escape from CyberCity (USA).chd` (DVC)
- `International Tennis Open (Europe).chd`
- `Kingdom - The Far Reaches (USA).chd` (DVC)
- `Tangram - The Ultimate Chinese Game (Europe).chd`
- `Ultra CD-i Soccer (Europe).chd`
