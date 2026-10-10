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
export const CLE_DU_FOND = "cassini-fond-de-recette";

/** Commande du moteur qui sert les octets de l'archive. */
const COMMANDE = "lire_le_fond_de_carte";

/**
 * Lecteur de l'archive locale pour la bibliothèque pmtiles.
 *
 * Le quatrième paramètre de `getBytes` (`etag`) est volontairement absent :
 * un fichier local servi par l'IPC n'a pas d'étiquette de version, et la
 * bibliothèque ne passe que ce qu'une source lui a d'abord renvoyé.
 */
export class SourceFondDeCarte implements Source {
  /**
   * @param signalerEchec reçoit le message du moteur quand la lecture échoue,
   * par exemple quand l'archive a été renommée ou supprimée.
   *
   * C'est le **seul** chemin par lequel un échec du fond de carte est rapporté.
   * L'événement `error` de MapLibre ne peut pas en tenir lieu : sa source est
   * marquée chargée avant que l'erreur ne soit émise, donc `load` suit
   * immédiatement et recouvrirait le message.
   */
  constructor(private readonly signalerEchec: (message: string) => void) {}

  getKey(): string {
    return CLE_DU_FOND;
  }

  async getBytes(decalage: number, longueur: number, signal?: AbortSignal): Promise<RangeResponse> {
    signal?.throwIfAborted();
    const octets = await this.demander(decalage, longueur);
    signal?.throwIfAborted();
    return { data: octets };
  }

  /** L'appel au moteur, isolé pour ne rapporter que ses échecs à lui. */
  private async demander(decalage: number, longueur: number): Promise<ArrayBuffer> {
    try {
      return await invoke<ArrayBuffer>(COMMANDE, { decalage, longueur });
    } catch (erreur) {
      const message = erreur instanceof Error ? erreur.message : String(erreur);
      this.signalerEchec(message);
      throw new Error(message);
    }
  }
}
