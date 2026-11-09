# ADR-0015: Grafica realistica e personaggi CC0

- Data: 2026-11-09
- Stato: accettato (Fase 1 di 3)
- Versione: da rilasciare (v0.0.25.0)

## Contesto
I personaggi erano parallelepipedi e tutto il mondo cel-shaded a colori di vertice. Si vuole un aspetto realistico PBR, in tre fasi: personaggi, mondo e materiali, animazioni.

## Decisione
- Si abbandona il cel-shading a favore di materiali PBR. Non è fotorealismo da tripla A.
- Asset CC0 di Quaternius, livello gratuito, scaricati una volta sola: Universal Base Characters, Modular Character Outfits – Fantasy, Universal Animation Library. Tutto poi gira in locale.
  I grezzi stanno in `assets_src/` (fuori da git e da Drive); nel gioco va solo il risultato, in `assets/models/characters/`. Provenienza e licenze: `assets/models/LICENSES.md`.
- I tre pacchetti hanno lo stesso scheletro da 65 ossa con gli stessi nomi (verificato sui file): `scripts/build_characters.py` non usa Blender, unisce i glTF in puro Python. Ogni parte tiene le proprie matrici di bind,
  quindi la testa del corpo intero (tagliata per peso delle ossa `Head`/`neck_01`) e i capelli seguono le ossa dell'abito. Texture ridotte a 1024 px e condivise; varianti di colore generate dalle due texture per abito.
- Le clip (`animations.gltf`) sono filtrate: rotazioni di tutte le ossa e traslazione del solo bacino, così non deformano le proporzioni dei modelli.
- Ogni modello contiene un'animazione fittizia di un fotogramma: senza, Bevy non crea `AnimationPlayer` e bersagli.
- In gioco: `figure::spawn_character` aggiunge il modello come figlio del corpo (collider, targhette e routine non cambiano). Pronta la scena, parte la clip di riposo e `HandBones` rende disponibili le ossa delle mani
  ad armi, scudo, lancia e lanterna (`AttachToHand`). Se un file manca: errore nel log e figura a blocchi.

## Conseguenze
- Il livello gratuito ha solo due abiti (contadino, ranger): la varietà viene da tinte e capelli.
- I personaggi costano 18–33 mila triangoli l'uno; misurati 49–71 FPS con tutti in scena (RTX 3080 Ti), in linea con prima.
- Finché non arriva la Fase 3 tutti stanno in posa di riposo: chi cammina scivola. La luce è ancora quella cel: la ricalibra la Fase 2.
