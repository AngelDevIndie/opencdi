Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$projectDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
if (-not $projectDir) { $projectDir = Get-Location }

# Finestra Principale
$form = New-Object System.Windows.Forms.Form
$form.Text = "OpenCDi – Philips CD-i Emulator Launcher"
$form.Size = New-Object System.Drawing.Size(520, 560)
$form.StartPosition = "CenterScreen"
$form.FormBorderStyle = "FixedDialog"
$form.MaximizeBox = $false
$form.BackColor = [System.Drawing.Color]::FromArgb(24, 24, 27) # Dark Zinc 900
$form.ForeColor = [System.Drawing.Color]::FromArgb(244, 244, 245)

# Titolo Header
$titleLabel = New-Object System.Windows.Forms.Label
$titleLabel.Text = "🎮 OpenCDi Emulator"
$titleLabel.Font = New-Object System.Drawing.Font("Segoe UI", 18, [System.Drawing.FontStyle]::Bold)
$titleLabel.ForeColor = [System.Drawing.Color]::FromArgb(239, 68, 68) # Red 500
$titleLabel.Size = New-Object System.Drawing.Size(480, 40)
$titleLabel.Location = New-Object System.Drawing.Point(20, 20)
$titleLabel.TextAlign = "MiddleCenter"
$form.Controls.Add($titleLabel)

# Sottotitolo
$subLabel = New-Object System.Windows.Forms.Label
$subLabel.Text = "Hotel Mario (USA) – Versione Interattiva"
$subLabel.Font = New-Object System.Drawing.Font("Segoe UI", 11, [System.Drawing.FontStyle]::Regular)
$subLabel.ForeColor = [System.Drawing.Color]::FromArgb(161, 161, 170) # Zinc 400
$subLabel.Size = New-Object System.Drawing.Size(480, 25)
$subLabel.Location = New-Object System.Drawing.Point(20, 60)
$subLabel.TextAlign = "MiddleCenter"
$form.Controls.Add($subLabel)

# 1. Pulsante Principale: AVVIA HOTEL MARIO
$btnPlay = New-Object System.Windows.Forms.Button
$btnPlay.Text = "▶  GIOCA A HOTEL MARIO"
$btnPlay.Font = New-Object System.Drawing.Font("Segoe UI", 14, [System.Drawing.FontStyle]::Bold)
$btnPlay.BackColor = [System.Drawing.Color]::FromArgb(34, 197, 94) # Green 500
$btnPlay.ForeColor = [System.Drawing.Color]::White
$btnPlay.FlatStyle = "Flat"
$btnPlay.FlatAppearance.BorderSize = 0
$btnPlay.Size = New-Object System.Drawing.Size(460, 55)
$btnPlay.Location = New-Object System.Drawing.Point(25, 105)
$btnPlay.Cursor = [System.Windows.Forms.Cursors]::Hand
$btnPlay.Add_Click({
    $exePath = Join-Path $projectDir "target\release\opencdi.exe"
    if (-not (Test-Path $exePath)) {
        $exePath = Join-Path $projectDir "opencdi.exe"
    }
    if (Test-Path $exePath) {
        $btnPlay.Enabled = $false
        $gameDisc = Join-Path $projectDir "giochi\Hotel Mario (USA).chd"
        if (-not (Test-Path $gameDisc)) {
            $gameDisc = Join-Path (Split-Path $projectDir -Parent) "file utili\giochi\Hotel Mario (USA).chd"
        }
        if (-not (Test-Path $gameDisc)) {
            $gameDisc = Join-Path $projectDir "file utili\giochi\Hotel Mario (USA).chd"
        }
        if (Test-Path $gameDisc) {
            Start-Process -FilePath $exePath -ArgumentList "`"$gameDisc`"" -WorkingDirectory $projectDir -Wait
        } else {
            Start-Process -FilePath $exePath -WorkingDirectory $projectDir -Wait
        }
        $btnPlay.Enabled = $true
        $btnPlay.Text = "▶  GIOCA A HOTEL MARIO"
    } else {
        [System.Windows.Forms.MessageBox]::Show("Eseguibile opencdi.exe non trovato! Ricompila con cargo build --release.", "Errore", [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Error)
    }
})
$form.Controls.Add($btnPlay)

# 2. Pulsante: APRI GALLERIA SCREENSHOT (Browser)
$btnGallery = New-Object System.Windows.Forms.Button
$btnGallery.Text = "🖼️  Apri Galleria nel Browser"
$btnGallery.Font = New-Object System.Drawing.Font("Segoe UI", 11, [System.Drawing.FontStyle]::Bold)
$btnGallery.BackColor = [System.Drawing.Color]::FromArgb(59, 130, 246) # Blue 500
$btnGallery.ForeColor = [System.Drawing.Color]::White
$btnGallery.FlatStyle = "Flat"
$btnGallery.FlatAppearance.BorderSize = 0
$btnGallery.Size = New-Object System.Drawing.Size(225, 45)
$btnGallery.Location = New-Object System.Drawing.Point(25, 175)
$btnGallery.Cursor = [System.Windows.Forms.Cursors]::Hand
$btnGallery.Add_Click({
    $htmlPath = Join-Path $projectDir "screenshots_hotel_mario\index.html"
    if (Test-Path $htmlPath) {
        Start-Process $htmlPath
    } else {
        [System.Windows.Forms.MessageBox]::Show("Galleria HTML non trovata!", "Avviso", [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Warning)
    }
})
$form.Controls.Add($btnGallery)

# 3. Pulsante: CARTELLA FOTO / SCREENSHOT
$btnFolder = New-Object System.Windows.Forms.Button
$btnFolder.Text = "📁  Cartella Immagini"
$btnFolder.Font = New-Object System.Drawing.Font("Segoe UI", 11, [System.Drawing.FontStyle]::Bold)
$btnFolder.BackColor = [System.Drawing.Color]::FromArgb(75, 85, 99) # Gray 600
$btnFolder.ForeColor = [System.Drawing.Color]::White
$btnFolder.FlatStyle = "Flat"
$btnFolder.FlatAppearance.BorderSize = 0
$btnFolder.Size = New-Object System.Drawing.Size(225, 45)
$btnFolder.Location = New-Object System.Drawing.Point(260, 175)
$btnFolder.Cursor = [System.Windows.Forms.Cursors]::Hand
$btnFolder.Add_Click({
    $folderPath = Join-Path $projectDir "screenshots_hotel_mario"
    if (Test-Path $folderPath) {
        Start-Process "explorer.exe" $folderPath
    }
})
$form.Controls.Add($btnFolder)

# Riquadro Controlli (Guida rapida a video)
$panelControls = New-Object System.Windows.Forms.GroupBox
$panelControls.Text = " Guida Rapida ai Controlli "
$panelControls.Font = New-Object System.Drawing.Font("Segoe UI", 10, [System.Drawing.FontStyle]::Bold)
$panelControls.ForeColor = [System.Drawing.Color]::FromArgb(250, 204, 21) # Yellow 400
$panelControls.Size = New-Object System.Drawing.Size(460, 230)
$panelControls.Location = New-Object System.Drawing.Point(25, 240)

$lblHelp = New-Object System.Windows.Forms.Label
$lblHelp.Text = "• FRECCE TASTIERA oppure MOUSE:`n   Muovi il cursore / D-Pad direzionale`n`n• TASTO Z / BARRA SPAZIATRICE / CLICK SINISTRO:`n   Pulsante 1 (Seleziona voce menu / Salto / Azione)`n`n• TASTO X / C / CLICK DESTRO:`n   Pulsante 2 (Azione secondaria)`n`n• TASTO S:`n   Scatta una foto istantanea (salvata in screenshots_hotel_mario)`n`n• TASTO ESC:`n   Chiudi l'emulatore"
$lblHelp.Font = New-Object System.Drawing.Font("Segoe UI", 9.5, [System.Drawing.FontStyle]::Regular)
$lblHelp.ForeColor = [System.Drawing.Color]::FromArgb(228, 228, 231)
$lblHelp.Size = New-Object System.Drawing.Size(430, 195)
$lblHelp.Location = New-Object System.Drawing.Point(15, 25)
$panelControls.Controls.Add($lblHelp)
$form.Controls.Add($panelControls)

# Mostra interfaccia
[void]$form.ShowDialog()
