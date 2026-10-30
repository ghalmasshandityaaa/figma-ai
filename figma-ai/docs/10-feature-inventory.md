# Kanvas — Daftar Fitur yang Direncanakan

Tanggal ringkasan: 2026-10-27.

Kanvas adalah editor desain berbasis browser seperti Figma, dengan AI agent yang bekerja langsung di canvas, terlihat prosesnya, dan bisa dihentikan pengguna.

**Status:** repository ini masih berisi spesifikasi produk, bukan aplikasi yang sudah diimplementasikan. Daftar berikut menjelaskan fitur yang akan dibuat, bukan fitur yang sudah tersedia. Ringkasan ini tidak mengubah scope atau menggantikan spesifikasi sumber.

Sumber utama: [README](../README.md), [Product Vision](./00-product-vision.md), dan [Implementation Plan](./09-implementation-plan.md).

## 1. Workspace dan Pengelolaan File

- Login menggunakan magic link dan Google.
- Workspace dan dashboard daftar file.
- Membuat, membuka, mengganti nama, menduplikasi, memindahkan, mengarsipkan, dan menghapus file.
- Tampilan All files, Shared with me, Templates, dan Trash.
- File desain dengan banyak halaman.
- Berbagi file melalui link dan pengaturan akses.
- Tema terang dan gelap untuk editor.

Sumber: [UX Architecture §2–§3](./01-ux-architecture.md) dan [Implementation Plan §5.2](./09-implementation-plan.md).

## 2. Canvas dan Manipulasi Objek

- Infinite canvas, pan, zoom, dan zoom ke selection.
- Ruler, guides, layout grid, dan pixel grid.
- Snapping dan pengukuran jarak antarobjek.
- Pemilihan objek melalui klik, multi-select, box selection, dan lasso.
- Drag, resize, rotate, flip, align, dan distribute.
- Group/ungroup, pengaturan urutan depan-belakang, dan pemindahan objek antarcontainer.
- Copy/paste, duplicate, rename, lock, dan hide.
- Undo/redo untuk perubahan manual.

Sumber: [Core Editor Feature Specification §2–§3](./03-feature-spec-core.md).

## 3. Drawing, Shape, dan Vector

- Frame dengan preset mobile, tablet, desktop, watch, dan presentation.
- Rectangle, rounded rectangle, ellipse, line, dan arrow.
- Triangle, diamond, polygon, dan star.
- Slice untuk menentukan area ekspor.
- Pen tool dan pengeditan titik serta kurva vector.
- Pencil/freeform drawing.
- Boolean operations: union, subtract, intersect, dan exclude.
- Flatten vector dan outline stroke, dengan batasan jenis stroke yang didukung.
- Impor SVG menjadi elemen yang dapat diedit, dengan diagnostic untuk fitur yang tidak didukung.

Sumber: [Core Editor Feature Specification §4–§5](./03-feature-spec-core.md).

## 4. Text dan Typography

- Teks satu atau banyak baris.
- Pengaturan font, style, ukuran, line height, letter spacing, dan alignment.
- Rich text: style berbeda dalam satu text layer.
- Pengaturan case, underline, strikethrough, list, dan hyperlink.
- Auto-width, auto-height, fixed-size, dan fill-width.
- Dukungan input IME.
- Deteksi missing font dan penggantian font.
- Metadata aksesibilitas seperti role dan heading level.

Sumber: [Core Editor Feature Specification §6](./03-feature-spec-core.md).

## 5. Auto Layout

- Layout horizontal dan vertikal.
- Gap, padding, alignment, dan distribution.
- Wrap dan nested auto layout.
- Ukuran Fixed, Hug, dan Fill.
- Minimum dan maksimum ukuran.
- Absolute-positioned children.
- Reorder elemen melalui canvas atau panel layer.
- Peringatan ketika layout overflow atau konfigurasi sizing bermasalah.

Sumber: [Core Editor Feature Specification §7](./03-feature-spec-core.md).

## 6. Layers dan Properties Panel

- Tree layer bertingkat dengan pencarian.
- Drag-and-drop untuk reorder dan reparent.
- Pengelolaan halaman: tambah, rename, duplicate, reorder, dan hapus.
- Pemindahan objek antarhalaman.
- Properties panel untuk posisi, ukuran, rotation, dan constraints.
- Fill solid, gradient, dan image.
- Stroke, opacity, blend mode, shadow, blur, dan corner radius.
- Pengeditan banyak objek sekaligus dengan indikator nilai Mixed.
- Command palette untuk mencari tindakan, layer, halaman, dan component.

Sumber: [Core Editor Feature Specification §8–§11](./03-feature-spec-core.md).

## 7. Components dan Variants

- Membuat component dari frame atau selection.
- Memasang instance, swap component, dan detach instance.
- Override teks, visual, visibility, dan properti instance.
- Reset override dan navigasi ke master component.
- Nested component.
- Variant sets dengan beberapa axis, misalnya ukuran dan state.
- Component properties: text, boolean, instance swap, variant, dan number.
- Component browser dengan pencarian dan preview.
- Internal library untuk penggunaan ulang di workspace.
- Publish flow dengan validasi, diff, versi, dan changelog.
- Proteksi master component dari perubahan agent.

Catatan fase: internal library masuk rencana MVP; TEAM library diklasifikasikan V1 dan EXTERNAL library V2 dalam spesifikasi sistem. Library publishing dengan semver juga tercantum sebagai fitur yang boleh digeser ke V1 jika diperlukan.

Sumber: [Feature Spec: Systems §2](./04-feature-spec-systems.md) dan [Implementation Plan §5.4, §7.2](./09-implementation-plan.md).

## 8. Design System dan Tokens

- Variables, collections, modes, dan themes.
- Token warna, spacing, radius, typography, shadow, dan kategori lainnya.
- Light/dark theme.
- Alias token dan binding token ke properti objek.
- Panel token dengan pencarian, filter, dan usage tracking.
- Deteksi hardcoded values, alias cycle, scope mismatch, dan masalah contrast.
- Impor/ekspor token melalui JSON dan CSS.
- Penggunaan token dan component oleh AI, bukan hanya menghasilkan bentuk visual.
- Multi-brand themes direncanakan dalam milestone design system, tetapi boleh digeser ke V1; light/dark tetap dipertahankan.

Sumber: [Feature Spec: Systems §3](./04-feature-spec-systems.md) dan [Implementation Plan §5.4, §7.2](./09-implementation-plan.md).

## 9. Asset Manager

- Upload gambar, SVG, dan font.
- Pencarian serta pengelompokan asset melalui folder dan tag.
- Drag asset ke canvas.
- Image fill, fit, crop, dan tile.
- Replace asset, termasuk penggantian di seluruh penggunaannya.
- Melihat lokasi penggunaan asset.
- Deduplication dan optimasi gambar.

Sumber: [Feature Spec: Systems §4](./04-feature-spec-systems.md) dan [Implementation Plan §5.3–§5.4](./09-implementation-plan.md).

## 10. Prototyping

- Menghubungkan frame menjadi alur interaktif.
- Trigger seperti click, hover, press, drag, keyboard, timeout, dan scroll.
- Action navigate, back, overlay, scroll, perubahan variable, perubahan variant, dan link.
- Transition instant, dissolve, move, push, slide, dan smart animate sesuai batas dukungan.
- Pengaturan overlay dan titik awal flow.
- Prototype player dengan present mode, device frame, back/restart, dan share link.
- Reduced-motion mode dan validasi alur.
- Komentar dalam preview dicantumkan dalam milestone prototype, meskipun product vision menempatkan komentar pada prototype di roadmap setelah MVP.

Catatan fase: smart animate boleh digeser ke V1; dissolve dan navigate tetap dipertahankan. Advanced prototyping dan kedalaman smart animate yang lebih luas termasuk tema post-MVP.

Sumber: [Feature Spec: Systems §5](./04-feature-spec-systems.md), [Product Vision §7](./00-product-vision.md), dan [Implementation Plan §5.8, §7.2](./09-implementation-plan.md).

## 11. Kolaborasi dan Version History

- Editing bersama secara realtime.
- Cursor, selection highlight, avatar, dan presence pengguna.
- Follow mode untuk mengikuti collaborator.
- Komentar pada objek atau posisi canvas.
- Thread, reply, mention, resolve, dan reopen.
- Role owner, editor, commenter, dan viewer.
- Version history otomatis maupun versi bernama.
- Preview perbedaan dan restore versi.
- Snapshot sebelum restore agar pemulihan versi tetap dapat dibatalkan.
- Activity log dan pemberitahuan konflik.
- Penyimpanan lokal serta pemulihan sinkronisasi saat koneksi terputus.
- Notifikasi komentar dan mention; in-app notifications inbox boleh digeser ke V1.

Catatan fase: offline mode disebut sebagai roadmap V1 dalam product vision, tetapi local persistence, reconnect, dan pengujian merge setelah offline sudah masuk milestone collaboration. Batas offline recovery dan offline mode penuh perlu diselaraskan sebelum scope implementasi dibekukan.

Sumber: [Feature Spec: Systems §6](./04-feature-spec-systems.md), [State Model §4–§6](./07-state-model.md), [Product Vision §7](./00-product-vision.md), dan [Implementation Plan §5.5, §7.2](./09-implementation-plan.md).

## 12. AI Agent

AI agent merupakan pembeda utama Kanvas: agent bekerja pada dokumen editable yang sama dengan pengguna, bukan menghasilkan gambar atau artefak terpisah yang tidak dapat diedit.

| Kelompok | Fitur yang direncanakan |
|---|---|
| Koneksi | Built-in agent, Claude Code, Codex/OpenAI-compatible, serta koneksi melalui MCP/local bridge dan custom MCP client. |
| Identitas dan sesi | Agent memiliki identitas, standby cursor, status, scope akses, serta sesi yang dapat dicabut. |
| Prompt | Prompt bar yang tetap terlihat, instruksi bahasa natural, konteks selection/page/file, attachments, slash commands, dan riwayat prompt. |
| Planning | Agent menampilkan rencana langkah sebelum atau selama eksekusi; meminta klarifikasi atau approval ketika diperlukan. |
| Eksekusi terlihat | Cursor agent memperlihatkan gerakan, hover, drag, typing, serta highlight objek yang berubah. |
| Pembuatan dan edit desain | Membuat frame, shape, text, layout; menggunakan component, token, dan asset; mengatur prototype serta menjalankan audit. |
| Kontrol pengguna | Pause, resume, stop, takeover, skip step, dan pengaturan kecepatan playback. |
| Explainability | Timeline tindakan, alasan perubahan, objek terdampak, diff, warning, dan ringkasan hasil. |
| Reversibility | Undo satu langkah, seluruh run, atau perubahan sejak langkah tertentu, dengan perlindungan terhadap edit lanjutan pengguna lain. |
| Mode eksekusi | Guided, supervised, watch, fast, dan read-only, sesuai policy file dan scope sesi. |
| Multi-agent | Pembagian tugas, reservation untuk mencegah tabrakan edit, handoff, dan reviewer read-only. |
| Safety | Scope akses, batas biaya/durasi/perubahan, konfirmasi tindakan destruktif, protected nodes, audit log, dan pencabutan sesi. |
| Aksesibilitas | Reduced-motion playback, kontrol keyboard, dan pengumuman status untuk screen reader. |

### Contoh Alur yang Ditargetkan

1. Pengguna meminta form login mobile melalui prompt.
2. Agent menampilkan rencana pengerjaan.
3. Agent membuat frame, judul, input email, input password, tombol login, dan link forgot password.
4. Agent menerapkan auto layout, spacing, component, dan token.
5. Pengguna melihat proses melalui cursor dan timeline, serta dapat menghentikan atau mengambil alih pekerjaan.
6. Agent menampilkan ringkasan perubahan dan warning.
7. Pengguna mengedit hasil secara manual, mengekspor, atau membatalkan langkah maupun seluruh run.

### Klarifikasi Fase AI

- Milestone M0–M8 berada dalam rencana menuju GA minggu ke-34. M5 bukan fase post-MVP.
- Integrasi Codex dan custom MCP dicantumkan sebagai jalur koneksi; tidak otomatis ditunda ke V1.
- Agent foundation berada di M4; pendalaman tools, MCP, multi-agent, dan mode eksekusi berada di M5.
- Watch mode dan UI orkestrasi multi-agent masuk rencana implementasi, tetapi boleh digeser ke V1 jika jadwal tertekan. Statusnya bukan otomatis sudah ditunda.
- Jika UI orkestrasi multi-agent digeser, implementation plan tetap mempertahankan protocol dan demo dua agent.
- Workspace-scoped sessions untuk pekerjaan agent lintas file disebut sebagai V1; sesi awal dibatasi pada satu file.

Sumber: [AI Agent Layer §3–§8](./05-ai-agent-layer.md), [UX Architecture §3.6, §5](./01-ux-architecture.md), dan [Implementation Plan §5.6–§5.7, §7.2](./09-implementation-plan.md).

## 13. Import, Export, dan Code Generation

- Impor gambar, SVG, token, dan struktur desain melalui Design JSON.
- Ekspor PNG, JPG, WebP, SVG, dan PDF.
- Scale presets, ekspor selection/frame, dan batch ZIP.
- Ekspor struktur desain dalam JSON.
- Ekspor token sebagai W3C/DTCG JSON, CSS variables, dan konfigurasi Tailwind.
- Generate React + TypeScript + Tailwind sebagai target kode utama.
- Generate HTML + CSS.
- Preview hasil, progress job, cancellation, dan download artifact.
- Diagnostic untuk fitur yang tidak dapat diekspor dengan fidelity yang sama.
- Hasil ekspor deterministik untuk input dan konfigurasi yang sama.
- Flutter dan React Native dijelaskan sebagai target dalam spesifikasi ekspor, tetapi ditempatkan pada roadmap post-MVP dan daftar fitur yang boleh digeser ke V1; jangan menganggapnya sebagai target MVP yang wajib tersedia.

Sumber: [Export & Code Generation](./08-export-and-codegen.md) dan [Implementation Plan §5.8, §7](./09-implementation-plan.md).

## 14. Fitur Operasional Menuju Peluncuran

- Onboarding, sample file, dan template gallery.
- Billing, subscription plan, quota, serta usage limits.
- Admin console untuk member, agent, dan penggunaan.
- Dokumentasi publik, support, changelog, dan status page.
- Materi peluncuran, demo, dan pricing page.

Pricing dan packaging masih hipotesis awal, bukan keputusan final.

Sumber: [Implementation Plan §5.10, §11](./09-implementation-plan.md).

## 15. Roadmap Setelah MVP

| Area | Fitur roadmap |
|---|---|
| Libraries | TEAM library yang lebih lengkap; EXTERNAL library lintas workspace pada fase berikutnya. |
| Code export | Flutter dan React Native export. |
| Agent orchestration | UI orkestrasi multi-agent yang lebih lengkap; sebagian scope juga direncanakan sebelum GA dan dapat digeser. |
| AI reviewers | Accessibility/motion reviewer khusus dan AI design reviewer yang memberikan komentar. |
| Responsive design | Responsive variant generation. |
| Prototyping | Advanced prototyping, interaction states yang lebih luas, dan runtime AI prototypes. |
| Tokens | Token migration tooling yang lebih lengkap. |
| Plugins | Plugin API dan ekosistem plugin. |
| Design-system publishing | Publishing dan marketplace yang lebih luas. |
| Code round-trip | Edit kode memperbarui desain. |
| Analytics | Design analytics untuk alur prototype. |
| Mobile | Native mobile apps dalam visi jangka panjang. |
| Enterprise | SSO/SCIM, DLP, retention, audit export, dan data residency. |

Product vision menempatkan V1 sekitar 3–6 bulan setelah MVP dan full vision pada horizon 12–24 bulan. Ini adalah horizon rencana, bukan bukti implementasi atau jaminan tanggal rilis.

Sumber: [Product Vision §7.2–§7.3](./00-product-vision.md), [Feature Spec: Systems §2.8](./04-feature-spec-systems.md), dan [README §5](../README.md).

## 16. Fitur yang Boleh Digeser ke V1

Implementation plan §7.2 mencantumkan daftar berikut sebagai opsi pengurangan scope, dalam urutan yang ditulis di sumber. Daftar ini berarti boleh digeser jika diperlukan, bukan sudah diputuskan ditunda.

1. Flutter / React Native export.
2. Multi-agent orchestration UI, dengan protocol dan demo dua agent tetap dipertahankan.
3. Watch mode dengan staged suggestions.
4. Library publishing dengan semver; local components tetap dipertahankan dan publish dibatasi read-only sesuai rencana pengurangan scope.
5. Smart animate; dissolve dan navigate tetap dipertahankan.
6. Freeform/pencil tool.
7. Mobile/tablet viewer mode; beta dapat desktop-only dengan warning.
8. Multi-brand themes; light/dark tetap dipertahankan.
9. In-app notifications inbox; email tetap dipertahankan.
10. Internationalization di luar bahasa Inggris dan Indonesia.

Sumber: [Implementation Plan §7.2](./09-implementation-plan.md).

## 17. Bukan Target MVP dan Non-Goals

### Di Luar Target MVP

- Impor native `.fig`; disebut untuk evaluasi setelah GA, bukan kemampuan yang sudah tersedia.
- Mobile editing penuh; viewer saja yang dipertimbangkan pada scope awal, dan viewer pun boleh digeser.
- Enterprise SSO/SCIM dan kompleksitas procurement enterprise.
- Eksekusi plugin.

### Bukan Fokus Produk

- Photorealistic image generation sebagai produk utama.
- Hidden AI edits tanpa visibilitas pengguna.
- Chat-only design tanpa canvas editable.
- 3D, video editing, atau motion-graphics authoring.
- Hosting atau deployment aplikasi production milik pengguna; produk mengekspor hasil desain dan kode.
- General-purpose whiteboard dengan target paritas FigJam.

Sumber: [README §5](../README.md) dan [Product Vision §7.4](./00-product-vision.md).

## 18. Kualitas yang Tidak Boleh Dikorbankan

- Attribution dan undo/redo, termasuk undo pekerjaan agent.
- Protected masters dan enforcement scope akses agent.
- Stop, takeover, dan confirmation gates.
- Kebenaran auto layout horizontal/vertikal dasar.
- Text editing tanpa merusak input IME.
- Durabilitas data tanpa kehilangan perubahan secara diam-diam.
- Determinisme hasil ekspor.
- Authorization dan sanitasi SVG.
- Core flows yang dapat dioperasikan melalui keyboard dan reduced-motion agent playback.

Sumber: [Implementation Plan §7.1, §7.3](./09-implementation-plan.md).

## 19. Catatan Konsistensi Scope

- Offline mode disebut pada roadmap V1 dalam product vision, tetapi local persistence dan offline recovery sudah masuk milestone collaboration.
- Komentar pada prototype disebut pada roadmap V1 dalam product vision, tetapi komentar dalam preview juga masuk milestone prototype player.
- Multi-agent orchestration UI muncul sebagai tema post-MVP di ringkasan produk, sementara milestone agent depth memuat coordination dan daftar scope control menyebut UI tersebut boleh digeser ke V1.
- Library publishing dan smart animate muncul dalam scope MVP, tetapi juga termasuk kandidat pengurangan scope.
- Keberadaan contoh TypeScript, schema, target emitter, atau CLI command di dokumen bukan bukti bahwa implementasinya sudah ada.

Batas-batas tersebut perlu diselaraskan ketika scope implementasi dibekukan. Ringkasan ini mempertahankan perbedaan antara fitur yang direncanakan, fitur yang boleh digeser, dan fitur yang secara eksplisit berada pada roadmap berikutnya.

## 20. Peta Dokumen Sumber

| Dokumen | Isi utama |
|---|---|
| [README](../README.md) | Definisi produk, status repository, ringkasan scope, dan known gaps. |
| [00 — Product Vision](./00-product-vision.md) | Visi, pengguna, prinsip, use cases, scope MVP, dan roadmap. |
| [01 — UX Architecture](./01-ux-architecture.md) | Struktur layar, panel, interaksi, prompt bar, dan agent UI. |
| [02 — Data Model](./02-data-model.md) | Istilah kanonis, entity, tipe, dan invariant data. |
| [03 — Core Editor](./03-feature-spec-core.md) | Canvas, shape, vector, text, auto layout, layers, pages, dan properties. |
| [04 — Systems](./04-feature-spec-systems.md) | Components, tokens, assets, prototype, collaboration, permissions, dan import/export UI. |
| [05 — AI Agent Layer](./05-ai-agent-layer.md) | Koneksi, planning, tools, cursor, kontrol, multi-agent, dan safety. |
| [06 — Architecture](./06-architecture.md) | Arsitektur client/backend, rendering, document engine, sync, dan gateway. |
| [07 — State Model](./07-state-model.md) | Commands, state, history, undo/redo, konflik, dan persistence. |
| [08 — Export and Codegen](./08-export-and-codegen.md) | Format ekspor, token output, code generation, fidelity, dan determinisme. |
| [09 — Implementation Plan](./09-implementation-plan.md) | Milestone M0–M8, scope control, quality gates, rollout, dan roadmap. |
