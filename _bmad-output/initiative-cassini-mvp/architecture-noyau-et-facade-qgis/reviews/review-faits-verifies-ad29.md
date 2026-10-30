---
type: review
lens: faits vérifiés (recherche web et confrontation au dépôt)
cible:
  - architecture-noyau-et-facade-qgis.md — précisions du 2026-10-30 de AD-2, AD-13, AD-14 ; nouvelle AD-29
  - change-ia-externe-pilote-la-session-qgis-ouverte.md
date: 2026-10-30
verdict: "à reprendre sur deux points avant 5.24 (méthodes « existantes » et tokio sans `net`) ; les faits d'OS et de clients MCP tiennent"
---

# Revue « faits vérifiés » — amendement AD-29 (2026-10-30)

Lentille : chaque affirmation technique engagée par l'amendement a-t-elle été vérifiée sur le web ou
dans le dépôt, plutôt qu'affirmée de mémoire ? Le document architectural n'a pas été modifié.

## Verdict

Les faits extérieurs tiennent : le tube nommé Windows sait refuser les clients distants et se
restreindre par descripteur de sécurité sans Winsock ; la socket de domaine Unix est locale ; Claude
Desktop, ChatGPT pour ordinateur / Codex CLI et Gemini CLI lancent bien un serveur MCP local sur stdio,
et ChatGPT web non ; `rmcp` 3.8.0 connaît bien la révision 2026-07-28.

Deux affirmations, en revanche, sont **fausses ou trompeuses au regard du dépôt** et doivent être
corrigées avant que 5.24 ne parte : (1) AD-29 dit que le relais n'emploie « que les méthodes
existantes » et que `session.busy` couvre « un plan » — le code actuel ne le permet pas tel quel ;
(2) le canal local ne peut pas s'écrire avec `tokio::net`, que `deny.toml` interdit, et ni AD-14 ni la
proposition ne le disent.

## Tableau des affirmations

| # | Affirmation | Où | Statut | Preuve |
|---|---|---|---|---|
| 1 | Le tube nommé Windows peut refuser les clients distants | AD-2, AD-29, proposition §4 | **Vérifié** | `PIPE_REJECT_REMOTE_CLIENTS` (0x8) : « Connections from remote clients are automatically rejected » [S1] |
| 2 | … et n'admettre que l'utilisateur courant | AD-2 | **Vérifié, avec réserve** | Le DACL se passe dans `lpSecurityAttributes`. Le descripteur **par défaut** donne la lecture à *Everyone* et au compte anonyme [S1][S2] : il faut donc impérativement en fournir un. Microsoft recommande le **SID de logon** plutôt que le SID utilisateur pour exclure aussi les autres sessions (RDP) du même compte [S2] |
| 3 | Le tube nommé passe par `kernel32`, la garde « aucune DLL réseau » reste valable | proposition §2 « Impact technique » | **Vérifié, incomplet** | `CreateNamedPipeW` est dans Kernel32.dll [S1]. Mais construire le descripteur (`ConvertStringSecurityDescriptorToSecurityDescriptorW`, ou lecture du jeton) importe **Advapi32.dll** [S3]. Advapi32 n'est pas dans la liste `FORBIDDEN` de `cli/tests/hors_ligne.rs` (ws2_32, wsock32, winhttp, wininet, mswsock, dnsapi) : la garde reste verte, mais la phrase « par l'API Win32 (kernel32) » est à compléter |
| 4 | Ne pas utiliser AF_UNIX sous Windows (choix implicite du tube nommé) | AD-2 | **Vérifié, bon choix** | AF_UNIX sous Windows passe par l'API Winsock [S4] : il importerait `ws2_32` et ferait tomber `hors_ligne.rs`. Ce motif mériterait d'être écrit dans AD-2 pour qu'un ticket ne « simplifie » pas en AF_UNIX partout |
| 5 | Socket de domaine Unix dans un dossier privé : locale seulement | AD-2 | **Vérifié, avec réserves** | AF_UNIX sert « between processes on the same machine » ; sous Linux les sockets à chemin respectent les droits du dossier [S5]. Réserves : (a) certains systèmes (anciens BSD) ignorent les droits du fichier socket — c'est bien le **dossier 0700** qui protège, pas le mode du fichier [S5] ; (b) l'espace de noms **abstrait** de Linux n'a aucun droit — à proscrire explicitement [S5] ; (c) `sun_path` limité à 108 octets sous Linux, parfois 92–104 ailleurs [S5] : un chemin sous un dossier utilisateur long peut échouer |
| 6 | Claude Desktop lance un serveur MCP local sur stdio | proposition §1, AD-29 | **Vérifié** | `claude_desktop_config.json`, `mcpServers` / `command` / `args`, Windows `%APPDATA%\Claude\` [S6] ; extensions `.mcpb` en plus [S7] |
| 7 | ChatGPT pour ordinateur et Codex CLI, configuration partagée `[mcp_servers.<nom>]`, `command` + `args` | proposition §1 | **Vérifié** | « The ChatGPT desktop app, Codex CLI, and IDE extension share this configuration », `~/.codex/config.toml`, STDIO = « Servers that run as a local process » [S8][S9]. **Non confirmé** par la page officielle : la disponibilité de l'application de bureau **sous Windows** avec serveurs STDIO ; des sources secondaires l'indiquent [S12]. À vérifier sur le poste du propriétaire au début de 5.26 |
| 8 | Gemini CLI, `mcpServers` dans `settings.json` | proposition §1 | **Vérifié** | Stdio choisi quand `command` est donné ; `args`, `env`, `cwd`, `timeout` (600 000 ms par défaut), `trust` [S10] |
| 9 | L'application web de ChatGPT ne lance pas de processus local | proposition §1, décision 2 | **Vérifié** | « ChatGPT web uses remote MCP tools through plugins » et « doesn't read local Codex configuration files » [S8] |
| 10 | `rmcp` 3.8.0 sert la révision 2026-07-28 | AD-3, pile, « Révision de MCP servie » | **Vérifié dans la source** | `src/model.rs` : `LATEST = V_2026_07_28`, `LATEST_WITH_INITIALIZE = V_2025_11_25`, `KNOWN_VERSIONS` de 2024-11-05 à 2026-07-28 ; la feature `server` ne tire que `transport-async-rw` (pas `transport-io`, qui tirerait `tokio/net`) — conforme à ce qu'écrit le document. La page officielle MCP renvoie bien à `docs/2026-07-28/` [S6] |
| 11 | `tokio` admis sous `rmcp`, feature `net` interdite (AD-14), **et** canal local dans `serve`/`mcp` | AD-14, AD-29 | **Contradiction non signalée** | `tokio::net::windows::named_pipe` est « Available on Windows and crate feature `net` only » [S11] ; il en va de même de `tokio::net::UnixStream`. `deny.toml` (`[[bans.features]] crate = "tokio" deny = ["net"]`) refusera donc toute implémentation par tokio. Voir constat C2 |
| 12 | Le relais n'emploie que `session.hello`, `command.run`, `request.cancel` | AD-29 | **Méthodes présentes, sémantique inadaptée** | Voir constat C1 |
| 13 | Un appel externe pendant un plan ou un autre appel reçoit `session.busy`, jamais mis en file | AD-29 | **Partiellement existant** | Voir constat C1 |
| 14 | « Un test montre qu'il refuse un client distant » | AD-14 | **Faisabilité non vérifiée** | Aucune source trouvée sur le comportement d'un client en boucle locale `\\localhost\pipe\…` ou `\\127.0.0.1\pipe\…` (chemin SMB) face à `PIPE_REJECT_REMOTE_CLIENTS`, ni sur la présence du service Serveur sur les exécuteurs Windows de GitHub. À prototyper avant d'écrire le critère de 5.24 |

## Constats

### C1 — `high` — « Aucune méthode nouvelle » : vrai pour les noms, faux pour le comportement

Les trois méthodes et le code `session.busy` existent bien dans `core/src/serve.rs` (lignes 340–460 et
562 sq.). Mais :

- **`session.hello` ne peut pas servir au relais tel quel.** `hello()` refuse un second appel
  (`session.already_started`, l. 563–567) et exige `commands` — le catalogue **que l'extension annonce**
  (commandes QGIS qu'elle exécute) — et `project`. Le relais n'a ni l'un ni l'autre ; il doit recevoir le
  catalogue, pas l'annoncer. Il faut donc soit un état par connexion distinct (et une poignée de main du
  relais aux paramètres différents), soit une méthode nouvelle. Écrire « aucune méthode nouvelle » fige
  une promesse que le code ne peut pas tenir sans changer le contrat de `session.hello`, ce qui touche
  AD-7.
- **`session.busy` n'existe que pendant un ordre QGIS en vol.** Il est rendu par
  `Channel::answer_during_order` (l. 446–466), c'est-à-dire seulement quand le noyau attend un
  `order.execute`. Pendant une étape `runs_in: core` d'un `plan.execute`, le fil principal ne lit pas :
  les lignes s'accumulent dans le fil lecteur et sont traitées après — c'est exactement une **mise en
  file**, que AD-29 interdit. Et un plan **proposé mais pas encore validé** (`state.pending`, `plans.rs`
  l. 205) ne rend rien occupé : un `command.run` passe. La règle « un appel externe arrivé pendant un plan
  reçoit `session.busy` » est donc un comportement **nouveau**, pas un réemploi.
- **Toute cette mécanique est par flux.** Elle lit l'entrée standard de l'extension ; le relais arrive
  sur une seconde connexion. Il faut une boucle qui multiplexe deux sources, ce qu'AD-4 et AD-5 doivent
  couvrir explicitement.
- **Identifiants de requête.** `request.cancel` compare l'`id` reçu à `link.run_id` (l. 418). Les `id`
  JSON-RPC du relais et de l'extension vivent dans deux espaces distincts et peuvent coïncider : sans
  préfixe ou table de correspondance, une annulation de l'un peut viser l'appel de l'autre.

Correction proposée : remplacer « les seules méthodes existantes de AD-6 (…) — aucune méthode nouvelle »
par « les méthodes de AD-6 ; toute méthode ou tout paramètre ajouté pour le relais est versionné par
AD-7 », et écrire que l'état « occupé » couvre l'ordre en vol, l'étape noyau en cours **et** le plan en
attente de validation.

### C2 — `high` — Le canal local ne peut pas passer par `tokio::net`, et rien ne le dit

AD-14 interdit la feature `net` de tokio ; or les tubes nommés et les sockets Unix de tokio sont
derrière `net` [S11]. Le premier réflexe d'implémentation (`tokio::net::windows::named_pipe::ServerOptions`
dans le relais `rmcp`) sera refusé par `cargo-deny`. Les voies conformes : `std::os::unix::net`
(bibliothèque standard, bloquante) et, sous Windows, l'API Win32 par une crate d'accès (`windows-sys`,
MIT OR Apache-2.0 — licence à demander, comme la proposition le prévoit), avec un fil bloquant raccordé
au runtime par `spawn_blocking` ou un `(R, W)` asynchrone maison. À écrire dans la précision d'AD-14 :
« le canal local d'AD-29 s'écrit sans `tokio::net` ».

Note : l'avis GHSA-7rrj-xr53-82p7 [S13] montre que `tokio` a déjà réinitialisé silencieusement
`reject_remote_clients` à `false` (versions antérieures à 1.23.1 ; le dépôt est en 1.54.1, non
concerné). C'est un argument de plus pour poser le drapeau à la main et le tester.

### C3 — `medium` — « N'admet que l'utilisateur courant » ne couvre que le sens client → serveur

Le DACL protège le serveur contre un client étranger. Il ne protège pas `cassini mcp` contre un **faux
serveur** : le nom du tube étant prévisible (« un nom de canal par utilisateur »), un autre processus
peut le créer avant la session QGIS. Ce que la documentation offre et que l'amendement ne cite pas :

- côté serveur, `FILE_FLAG_FIRST_PIPE_INSTANCE` (la création échoue en `ERROR_ACCESS_DENIED` si une
  instance existe [S1]) et `nMaxInstances = 1` — c'est aussi la mise en œuvre naturelle de « une
  seconde session QGIS est refusée » ;
- côté client, `SECURITY_SQOS_PRESENT` dans `CreateFile` pour limiter l'usurpation d'identité par le
  serveur [S14], et la vérification que le serveur appartient bien à l'utilisateur.

Sous Unix, l'équivalent est le dossier 0700 créé et vérifié (propriétaire, mode) avant `bind`, et le
nettoyage d'un fichier socket orphelin après un plantage (sinon `bind` échoue et la session croit à tort
qu'une autre existe).

### C4 — `medium` — Fait ChatGPT pour ordinateur sous Windows : à confirmer sur le poste

La page officielle confirme la configuration partagée et STDIO dans l'application de bureau [S8][S9]
mais ne dit pas les plateformes. Le propriétaire travaille sous Windows ; le critère de succès de
5.26 (« Claude, puis ChatGPT, puis Gemini ») en dépend. Ajouter à 5.26 une vérification de départ, avec
Codex CLI comme repli déjà documenté.

### C5 — `low` — Sources de la proposition incomplètes

La section « Sources » ne cite rien pour Claude Desktop ni pour `rmcp`, et l'URL
`developers.openai.com/codex/mcp` redirige désormais (308) vers
`learn.chatgpt.com/docs/extend/mcp?surface=cli`. Ajouter [S6], [S9] et le chemin de la source `rmcp`
lue (`src/model.rs`, constantes `LATEST`, `LATEST_WITH_INITIALIZE`).

## Sources

- [S1] CreateNamedPipeA/W, Microsoft Learn — https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createnamedpipea
- [S2] Named Pipe Security and Access Rights — https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights
- [S3] ConvertStringSecurityDescriptorToSecurityDescriptorW (Advapi32.dll) — https://learn.microsoft.com/en-us/windows/win32/api/sddl/nf-sddl-convertstringsecuritydescriptortosecuritydescriptorw
- [S4] AF_UNIX comes to Windows (Winsock) — https://devblogs.microsoft.com/commandline/af_unix-comes-to-windows/
- [S5] unix(7) — https://man7.org/linux/man-pages/man7/unix.7.html
- [S6] Connect to local MCP servers (Claude Desktop) — https://modelcontextprotocol.io/docs/develop/connect-local-servers
- [S7] Getting started with local MCP servers on Claude Desktop — https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop
- [S8] ChatGPT, MCP — https://learn.chatgpt.com/docs/extend/mcp
- [S9] Codex CLI, MCP (cible de la redirection de developers.openai.com/codex/mcp) — https://learn.chatgpt.com/docs/extend/mcp?surface=cli
- [S10] Gemini CLI, MCP servers — https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md
- [S11] tokio::net::windows::named_pipe (feature `net`) — https://docs.rs/tokio/latest/tokio/net/windows/named_pipe/index.html
- [S12] Source secondaire, ChatGPT de bureau et `~/.codex/config.toml` — https://docs.litellm.ai/docs/proxy/client_setup/codex_chatgpt_desktop.md ; https://www.windowsmode.com/chatgpt-for-windows/
- [S13] GHSA-7rrj-xr53-82p7 (tokio, reject_remote_clients) — https://github.com/advisories/GHSA-7rrj-xr53-82p7
- [S14] Impersonating a Named Pipe Client — https://learn.microsoft.com/en-us/windows/win32/ipc/impersonating-a-named-pipe-client
- Source `rmcp` lue localement : `/tmp/claude-1000/rmcp/rmcp-3.8.0/src/model.rs` (l. 167–207), `Cargo.toml` (features `server`, `transport-io`)
- Dépôt : `core/src/serve.rs`, `core/src/serve/plans.rs`, `cli/tests/hors_ligne.rs`, `cli/Cargo.toml`, `deny.toml`
