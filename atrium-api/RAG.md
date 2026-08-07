# RAG Atrium

1. Un administrateur importe les sources approuvees : regles, FAQ, guides et
   descriptions de salons. Les messages Discord des membres ne sont jamais
   indexes automatiquement.
2. Chaque source est decoupee en fragments de 300 a 500 tokens avec un leger
   recouvrement, puis chaque fragment recoit un embedding.
3. A chaque question, Atrium calcule l'embedding de la question et recupere
   les 3 a 5 fragments les plus proches, filtres par `guild_id`.
4. Seuls ces extraits, leur titre et leur URL sont fournis a DeepSeek. Si aucun
   score fiable n'est trouve, le bot indique qu'il ne sait pas et oriente vers
   la moderation.

## Configuration d'embeddings

Utiliser Ollama local (compatible OpenAI), distinct de DeepSeek :

```env
ATRIUM_EMBEDDINGS_BASE_URL=http://ollama:11434/v1
ATRIUM_EMBEDDINGS_API_KEY=
ATRIUM_EMBEDDINGS_MODEL=nomic-embed-text
ATRIUM_RAG_DATABASE_URL=postgres://...
```

La migration `002_ollama_nomic_embeddings.sql` adapte l'index a `vector(768)`
pour ce modele. Tout changement de modele et de dimension doit passer par une
nouvelle migration, jamais par la modification d'une migration deja appliquee.
