import { invoke } from "@tauri-apps/api/core";
import type { RangeResponse, Source } from "pmtiles";

// Source PMTiles branchée sur l'IPC Tauri.
//
// L'archive du fond de carte vit sur le disque et le moteur Rust est seul à la
// lire : l'interface demande des plages d'octets par un appel de commande, et
// jamais par le réseau (CLAUDE.md §2.3). Il n'y a donc ni `fetch`, ni URL, ni
// origine HTTP ajoutée au CSP de la fenêtre — l'IPC y est déjà autorisé.

/**
 * Clé de l'archive dans le protocole `pmtiles` de MapLibre.
 *
 * **Unique endroit du code qui porte cette clé.** `basemap.style.json` la
 * répète dans l'adresse de sa source, et `scripts/check-offline.mjs` relit la
 * présente ligne pour vérifier que les deux concordent octet pour octet.
 *
 * Ce contrôle n'est pas une coquetterie : le `Protocol` de la bibliothèque
 * pmtiles traite ce qui suit `pmtiles://` comme une clé de dictionnaire, et une
 * clé qu'il ne connaît pas le fait fabriquer un `FetchSource`, c'est-à-dire une
 * requête réseau, sans un mot dans la console.
 */
export const BASEMAP_KEY = "cassini-basemap-fixture";

/** Commande du moteur qui sert les octets de l'archive. */
const COMMAND = "read_basemap";

/**
 * Lecteur de l'archive locale pour la bibliothèque pmtiles.
 *
 * Le quatrième paramètre de `getBytes` (`etag`) est volontairement absent :
 * un fichier local servi par l'IPC n'a pas d'étiquette de version, et la
 * bibliothèque ne passe que ce qu'une source lui a d'abord renvoyé.
 */
export class BasemapSource implements Source {
  /**
   * @param reportFailure reçoit le message du moteur quand la lecture échoue,
   * par exemple quand l'archive a été renommée ou supprimée.
   *
   * C'est le **seul** chemin par lequel un échec du fond de carte est rapporté.
   * L'événement `error` de MapLibre ne peut pas en tenir lieu : sa source est
   * marquée chargée avant que l'erreur ne soit émise, donc `load` suit
   * immédiatement et recouvrirait le message.
   */
  constructor(private readonly reportFailure: (message: string) => void) {}

  getKey(): string {
    return BASEMAP_KEY;
  }

  async getBytes(offset: number, length: number, signal?: AbortSignal): Promise<RangeResponse> {
    signal?.throwIfAborted();
    const bytes = await this.request(offset, length);
    signal?.throwIfAborted();
    return { data: bytes };
  }

  /** L'appel au moteur, isolé pour ne rapporter que ses échecs à lui. */
  private async request(offset: number, length: number): Promise<ArrayBuffer> {
    try {
      return await invoke<ArrayBuffer>(COMMAND, { offset, length });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      this.reportFailure(message);
      throw new Error(message);
    }
  }
}
