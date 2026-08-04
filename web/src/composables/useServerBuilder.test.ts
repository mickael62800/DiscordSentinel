import { describe, expect, it } from "vitest";
import { nextTick } from "vue";
import type { PlanItem } from "@/services/guildStructureService";
import {
  planErrors,
  previewName,
  useServerBuilder,
  removeAccess,
  setAccess,
  TEMPLATES,
} from "./useServerBuilder";

describe("previewName", () => {
  it("normalise les salons écrits comme Discord le fera", () => {
    expect(previewName("Salon Général !", "text")).toBe("salon-général");
  });

  it("laisse les vocaux et catégories intacts", () => {
    expect(previewName("Salon Général !", "voice")).toBe("Salon Général !");
    expect(previewName("Ma Catégorie", "category")).toBe("Ma Catégorie");
  });

  it("rend vide un nom sans caractère utilisable", () => {
    expect(previewName("###", "text")).toBe("");
  });
});

describe("planErrors", () => {
  it("accepte un plan valide", () => {
    expect(planErrors([{ key: "a", name: "général", kind: "text" }])).toEqual([]);
  });

  it("refuse un nom vide", () => {
    expect(planErrors([{ key: "a", name: "  ", kind: "text" }])).toHaveLength(1);
  });

  it("refuse deux salons identiques au même endroit", () => {
    const errs = planErrors([
      { key: "a", name: "General", kind: "text" },
      { key: "b", name: "general", kind: "text" },
    ]);
    expect(errs).toHaveLength(1);
  });

  it("autorise le même nom sur deux types différents", () => {
    expect(
      planErrors([
        { key: "a", name: "general", kind: "text" },
        { key: "b", name: "general", kind: "voice" },
      ]),
    ).toEqual([]);
  });

  it("refuse plus de 50 salons dans une catégorie", () => {
    const items = [{ key: "cat", name: "Cat", kind: "category" as const }];
    for (let i = 0; i < 51; i++) {
      items.push({ key: `k${i}`, name: `salon-${i}`, kind: "text", parent_key: "cat" } as never);
    }
    expect(planErrors(items).some((e) => e.includes("maximum 50"))).toBe(true);
  });
});

describe("accès par rôle", () => {
  it("remplace le mode d'un rôle déjà réglé au lieu de le dupliquer", () => {
    const item: PlanItem = { key: "a", name: "salon", kind: "text" };
    setAccess(item, "7", "write");
    setAccess(item, "7", "read");
    expect(item.access).toEqual([{ role_id: "7", mode: "read" }]);
  });

  it("retire une règle", () => {
    const item: PlanItem = { key: "a", name: "salon", kind: "text" };
    setAccess(item, "7", "write");
    removeAccess(item, "7");
    expect(item.access).toEqual([]);
  });

  it("refuse privé et une règle @everyone ensemble", () => {
    const item = {
      key: "a",
      name: "salon",
      kind: "text" as const,
      private: true,
      access: [{ role_id: "42", mode: "read" as const }],
    };
    expect(planErrors([item], "42")).toHaveLength(1);
    // Sans le raccourci « privé », la règle explicite est parfaitement valide.
    expect(planErrors([{ ...item, private: false }], "42")).toEqual([]);
  });

  it("refuse deux règles sur le même rôle", () => {
    const item = {
      key: "a",
      name: "salon",
      kind: "text" as const,
      access: [
        { role_id: "7", mode: "read" as const },
        { role_id: "7", mode: "write" as const },
      ],
    };
    expect(planErrors([item], "42")).toHaveLength(1);
  });
});

describe("déplacement d'un salon", () => {
  it("range un salon dans une catégorie du plan, puis dans une catégorie existante", () => {
    const b = useServerBuilder();
    const cat = b.addCategory("Support");
    const chan = b.addChannel("text");
    expect(b.parentValue(chan)).toBe("");

    b.setParent(chan, `plan:${cat.key}`);
    expect(chan.parent_key).toBe(cat.key);
    expect(chan.parent_id).toBeNull();
    expect(b.childrenOf(cat.key)).toHaveLength(1);
    expect(b.parentValue(chan)).toBe(`plan:${cat.key}`);

    // Catégorie déjà présente sur le serveur : c'est un ID Discord.
    b.setParent(chan, "guild:555");
    expect(chan.parent_key).toBeNull();
    expect(chan.parent_id).toBe("555");
    expect(b.childrenOf(cat.key)).toHaveLength(0);

    b.setParent(chan, "");
    expect(chan.parent_key).toBeNull();
    expect(chan.parent_id).toBeNull();
  });
});

describe("persistance du plan", () => {
  // La sauvegarde passe par un watcher : elle s'exécute au tick suivant.
  it("retrouve le plan du serveur, et pas celui d'un autre", async () => {
    sessionStorage.clear();
    let guild = "A";
    const b = useServerBuilder(() => guild);
    b.addCategory("Support");
    b.addChannel("text");
    await nextTick();

    // Bascule sur un autre serveur : plan vierge.
    guild = "B";
    b.restore();
    expect(b.items.value).toEqual([]);

    // Retour sur le premier : le plan est intact.
    guild = "A";
    b.restore();
    expect(b.items.value).toHaveLength(2);
    expect(b.items.value[0].name).toBe("Support");
  });

  it("ne réutilise pas une clé après restauration", async () => {
    sessionStorage.clear();
    const b = useServerBuilder(() => "A");
    const first = b.addCategory("Cat");
    await nextTick();

    b.restore();
    const added = b.addChannel("text");
    expect(added.key).not.toBe(first.key);
    expect(new Set(b.items.value.map((i) => i.key)).size).toBe(b.items.value.length);
  });
});

describe("reprise après échec partiel", () => {
  it("rattache les salons restants à la catégorie réellement créée", () => {
    const b = useServerBuilder();
    const cat = b.addCategory("Support");
    const ok = b.addChannel("text", cat.key);
    const ko = b.addChannel("voice", cat.key);

    // La catégorie et un salon sont passés ; le vocal a échoué.
    b.dropCreated([
      { key: cat.key, channel_id: "999" },
      { key: ok.key, channel_id: "1000" },
    ]);

    expect(b.items.value).toHaveLength(1);
    const rest = b.items.value[0];
    expect(rest.key).toBe(ko.key);
    // Le lien local est remplacé par l'ID Discord réel : le plan reste valide.
    expect(rest.parent_key).toBeNull();
    expect(rest.parent_id).toBe("999");
    expect(b.errors.value).toEqual([]);
    expect(b.canApply.value).toBe(true);
  });

  it("signale un salon devenu orphelin plutôt que de laisser valider", () => {
    const errs = planErrors([
      { key: "c", name: "salon", kind: "text", parent_key: "disparue" },
    ]);
    expect(errs).toHaveLength(1);
    expect(errs[0]).toContain("catégorie parente");
  });
});

describe("limite de participants", () => {
  it("traite un champ vidé comme « illimité », sans erreur", () => {
    const item = {
      key: "a",
      name: "Vocal",
      kind: "voice",
      user_limit: "" as unknown as number,
    } as PlanItem;
    expect(planErrors([item])).toEqual([]);

    const b = useServerBuilder();
    b.items.value.push(item);
    // "" partirait tel quel dans le JSON et ferait rejeter tout le plan.
    expect(b.payload()[0].user_limit).toBeNull();
  });

  it("refuse une valeur hors bornes Discord", () => {
    const item: PlanItem = { key: "a", name: "Vocal", kind: "voice", user_limit: 150 };
    expect(planErrors([item])).toHaveLength(1);
  });
});

describe("useServerBuilder", () => {
  it("retire les salons d'une catégorie supprimée", () => {
    const b = useServerBuilder();
    const cat = b.addCategory("Support");
    b.addChannel("text", cat.key);
    b.addChannel("voice", cat.key);
    expect(b.items.value).toHaveLength(3);

    b.remove(cat.key);
    expect(b.items.value).toEqual([]);
  });

  it("sépare les salons sans catégorie", () => {
    const b = useServerBuilder();
    const cat = b.addCategory();
    b.addChannel("text", cat.key);
    b.addChannel("text");
    expect(b.childrenOf(cat.key)).toHaveLength(1);
    expect(b.rootChannels.value).toHaveLength(1);
  });

  it("bloque la validation d'un plan vide ou fautif", () => {
    const b = useServerBuilder();
    expect(b.canApply.value).toBe(false);

    const item = b.addChannel("text");
    expect(b.canApply.value).toBe(true);

    item.name = "###";
    expect(b.canApply.value).toBe(false);
  });

  it("produit des clés uniques entre modèles cumulés", () => {
    const b = useServerBuilder();
    b.applyTemplate(TEMPLATES[0]);
    b.applyTemplate(TEMPLATES[1]);
    const keys = b.items.value.map((i) => i.key);
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("livre des modèles dont chaque salon pointe une catégorie du plan", () => {
    for (const tpl of TEMPLATES) {
      const b = useServerBuilder();
      b.applyTemplate(tpl);
      const keys = new Set(b.items.value.map((i) => i.key));
      for (const item of b.items.value) {
        if (item.parent_key) expect(keys.has(item.parent_key)).toBe(true);
      }
      expect(planErrors(b.items.value)).toEqual([]);
    }
  });

  it("nettoie le plan à l'envoi", () => {
    const b = useServerBuilder();
    const item = b.addChannel("text");
    item.name = "  mon-salon  ";
    const [sent] = b.payload();
    expect(sent.name).toBe("mon-salon");
    expect(sent.topic).toBeNull();
    expect(sent.slowmode).toBe(0);
    expect(sent.access).toEqual([]);
  });

  it("transmet les accès réglés", () => {
    const b = useServerBuilder();
    const item = b.addChannel("text");
    setAccess(item, "7", "moderate");
    expect(b.payload()[0].access).toEqual([{ role_id: "7", mode: "moderate" }]);
  });
});
