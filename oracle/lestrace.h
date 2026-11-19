/* LOQ_TRACE_LES=1: log what the language module's functions get and give,
 * to <prefix>.les. LTTS7<Language>.dll's one export hands the engine a
 * name -> function table (LTTS7Italian.dll 0x10031ff8: 19 pairs; others via
 * LOQ_LES_DLL=LTTS7German.dll LOQ_LES_TABLE=100294a8); the module is loaded
 * before the voice and every slot swapped for a thunk that logs before and
 * after the real function (arguments, ctx dwords 0-4 — ctx+8 is the
 * sentence type — every 24-byte word record with its strings, FonTrasParola's
 * word and output codes, FonTrasConfini's code and counts):
 *   > <name> <6 argument dwords> [word=… idx=… | ctx=…]
 *     W <i> <24 record bytes> s4="text" …
 *   < <name> rc=<eax> [out=<codes>]
 * Up to 64 slots (NSLOT). */
FILE *leslog;
#define NSLOT 64
void *les_real[NSLOT];
static const char *les_name[NSLOT];
static unsigned les_args[NSLOT][8];
void *les_ret[NSLOT];
static unsigned les_base;

static void les_str(const char *tag, const unsigned char *s) {
    fprintf(leslog, " %s=", tag);
    if (!s || IsBadStringPtrA((const char *)s, 256)) { fprintf(leslog, "-"); return; }
    fputc('"', leslog);
    for (; *s; s++) {
        if (*s == '"' || *s == '\\') fprintf(leslog, "\\%c", *s);
        else if (*s < 0x20 || *s >= 0x7f) fprintf(leslog, "\\x%02x", *s);
        else fputc(*s, leslog);
    }
    fputc('"', leslog);
}

/* LOQ_LES_RU=1: LTTS7Russian.dll's LexInfo behind a record's +0x14
 * (LesAllocLanguageLexInfo 0x10009eb0: [0] the analyses, [3] the stressed
 * vowel LesAccenta found, +0x10 the stressed spelling): "  L pos=… form=…
 * n=… cur=…" then one "  A" line per analysis (by rank) with its segments
 * start-end:<spelling VA>/<id>/<class>. */
static void les_ru_lexinfo(unsigned li) {
    if (li < 0x10000 || IsBadReadPtr((void *)li, 0x54)) return;
    unsigned *L = (unsigned *)li;
    unsigned a = L[0];
    fprintf(leslog, "  L pos=%d", (int)L[3]);
    les_str("form", (unsigned char *)(li + 0x10));
    if (a < 0x10000 || IsBadReadPtr((void *)a, 0x50c)) { fprintf(leslog, "\n"); return; }
    unsigned n = *(unsigned *)(a + 4);
    fprintf(leslog, " n=%u cur=%d\n", n, (int)L[1]);
    for (unsigned k = 0; k < n && k < 10; k++) {
        unsigned s = *(unsigned *)(a + 0x4e0 + 4 * k);
        if (s >= 10) break;
        unsigned char *A = (unsigned char *)(a + 8 + s * 0x7c);
        unsigned m = *(unsigned *)(A + 0x78);
        fprintf(leslog, "  A %u slot=%u", k, s);
        for (unsigned j = 0; j < m && j < 10; j++) {
            unsigned *g = (unsigned *)(A + 12 * j);
            unsigned *c = (unsigned *)g[2];
            if (!c || IsBadReadPtr(c, 0x14)) { fprintf(leslog, " %d-%d:?", (int)g[0], (int)g[1]); continue; }
            fprintf(leslog, " %d-%d:%x/%d/%d", (int)g[0], (int)g[1], c[0] ? c[0] - les_base + 0x10000000 : 0, (int)c[2], (int)*(short *)((char *)c + 0x10));
        }
        fprintf(leslog, "\n");
    }
}

/* LOQ_LES_DA=1: LTTS7Danish.dll's LexInfo behind a record's +0x14 (0x294
 * bytes, LesAllocLanguageLexInfo 0x10002630): its first 0x14c bytes, the
 * per-letter table +0x14c (64 x {i8 morpheme, u8, u16 flags}) and the
 * marked spelling +0x250, then the solutions LesElaboraParola copied into
 * the buffer at +0 ({count, 10 x 0x194-byte solutions}; a solution is 10 x
 * 0x28-byte morphemes, getMorphemeInfo's 10 fields, and their count at
 * +0x190):
 *   L <hex 0x14c> | <per-letter hex> | sp="spelling"
 *   S <solution> <count>
 *   M <morpheme> s0="…" s4="…" sc="…" p8=<8 bytes> <+0x10 short> <+0x14 VA> <+0x18 … +0x24> */
static void les_da_lexinfo(unsigned li) {
    if (li < 0x10000 || IsBadReadPtr((void *)li, 0x294)) return;
    const unsigned char *L = (const unsigned char *)li;
    fprintf(leslog, "  L ");
    for (int k = 0; k < 0x14c; k++) fprintf(leslog, "%02x", L[k]);
    fprintf(leslog, " | ");
    for (int k = 0x14c; k < 0x24c; k++) fprintf(leslog, "%02x", L[k]);
    fprintf(leslog, " |");
    les_str("sp", L + 0x250);
    fprintf(leslog, "\n");
    unsigned buf = *(unsigned *)L;
    if (buf < 0x10000 || IsBadReadPtr((void *)buf, 0xfd0)) return;
    unsigned ns = *(unsigned *)buf;
    for (unsigned s = 0; s < ns && s < 10; s++) {
        const unsigned char *sol = (const unsigned char *)buf + 4 + s * 0x194;
        unsigned nm = *(unsigned *)(sol + 0x190);
        fprintf(leslog, "  S %u %u\n", s, nm);
        for (unsigned m = 0; m < nm && m < 10; m++) {
            const unsigned *f = (const unsigned *)(sol + m * 0x28);
            fprintf(leslog, "  M %u", m);
            les_str("s0", (const unsigned char *)f[0]);
            les_str("s4", (const unsigned char *)f[1]);
            les_str("sc", (const unsigned char *)f[3]);
            fprintf(leslog, " p8=");
            if (f[2] > 0x10000 && !IsBadReadPtr((void *)f[2], 8))
                for (int k = 0; k < 8; k++) fprintf(leslog, "%02x", ((unsigned char *)f[2])[k]);
            else fprintf(leslog, "-");
            fprintf(leslog, " %d %x %d %d %d %d\n", (short)f[4], f[5] ? f[5] - les_base + 0x10000000 : 0, (int)f[6], (int)f[7], (int)f[8], (int)f[9]);
        }
    }
}

/* set around LesAnalizza's and LesAccenta's record dumps: the Danish
   LexInfo is dumped there (and at FonTrasParola) only */
static int les_da_on;
/* the word records LES works on: ctx+4 -> 24-byte records, ctx+0xc count */
static void les_words(unsigned ctx) {
    if (!ctx || IsBadReadPtr((void *)ctx, 16)) return;
    unsigned char *rec = *(unsigned char **)(ctx + 4);
    unsigned n = *(unsigned short *)(ctx + 0xc);
    if (!rec || n > 4096 || IsBadReadPtr(rec, n * 24)) return;
    for (unsigned i = 0; i < n; i++) {
        unsigned char *r = rec + 24 * i;
        fprintf(leslog, "  W %u", i);
        for (int k = 0; k < 24; k++) fprintf(leslog, "%s%02x", k % 4 ? "" : " ", r[k]);
        for (int k = 0; k < 24; k += 4) {
            unsigned p = *(unsigned *)(r + k);
            if (p > 0x10000 && !IsBadStringPtrA((char *)p, 256) && !IsBadReadPtr((char *)p, 1)) {
                char t[8];
                snprintf(t, sizeof t, "s%d", k);
                les_str(t, (unsigned char *)p);
            }
        }
        fprintf(leslog, "\n");
        if (getenv("LOQ_LES_RU")) les_ru_lexinfo(*(unsigned *)(r + 0x14));
        if (getenv("LOQ_LES_DA") && les_da_on) les_da_lexinfo(*(unsigned *)(r + 0x14));
    }
}

static int les_nphones(unsigned ctx) {
    if (!ctx || IsBadReadPtr((void *)ctx, 0x58)) return 0;
    unsigned **arr = *(unsigned ***)(ctx + 0x10);
    int k = 0;
    if (!arr) return 0;
    for (; k < 1024; k++) {
        if (IsBadReadPtr(arr + k, 4) || !arr[k] || IsBadReadPtr(arr[k], 9) || !arr[k][1]) break;
    }
    return k;
}

static void les_phones(unsigned ctx) {
    if (!ctx || IsBadReadPtr((void *)ctx, 0x58)) return;
    unsigned **arr = *(unsigned ***)(ctx + 0x10);
    if (!arr) return;
    fprintf(leslog, "  P");
    for (int k = 0; k < 1024; k++) {
        if (IsBadReadPtr(arr + k, 4) || !arr[k] || IsBadReadPtr(arr[k], 9)) break;
        unsigned *e = arr[k];
        if (!e[1]) break;
        fprintf(leslog, " %u:%x:%02x", e[0], e[1], ((unsigned char *)e)[8]);
    }
    fprintf(leslog, "\n");
}


/* LesIniNumeri's slot 9 (+0x24): the engine's number reader, swapped for
   num_thunk after LesIniNumeri fills it. cdecl, 7 arguments: (ctx, type,
   idx, dmy, token buffer (0x40c-byte entries), out, text); returns char*. */
void *num_real, *num_ret;
static unsigned num_args[8];
static void num_toks(const char *pfx, unsigned char *tb, int idx) {
    if (!tb || IsBadReadPtr(tb, 0x40c)) return;
    for (int k = 0; k < 40; k++) {
        unsigned char *t = tb + (unsigned)k * 0x40c;
        if (IsBadReadPtr(t, 0x40c)) break;
        if (!t[0] && k > idx) break;
        fprintf(leslog, "  %s %d %d %d %d", pfx, k, *(int *)(t + 0x400), *(int *)(t + 0x404), t[0x408]);
        les_str("t", t);
        fprintf(leslog, "\n");
    }
}
void __cdecl num_pre(unsigned *args) {
    memcpy(num_args, args, sizeof num_args);
    fprintf(leslog, "> NUM type=%d idx=%d", (int)args[1], (int)args[2]);
    les_str("a3", (unsigned char *)args[3]);
    les_str("out", (unsigned char *)args[5]);
    les_str("a6", (unsigned char *)args[6]);
    unsigned c = args[0];
    if (c && !IsBadReadPtr((void *)c, 0x24)) {
        fprintf(leslog, " c18=%d", *(int *)(c + 0x18));
        unsigned *p20 = *(unsigned **)(c + 0x20);
        if (p20 && !IsBadReadPtr(p20, 4) && *p20 && !IsBadReadPtr((void *)(*p20 + 0x34), 4)) fprintf(leslog, " df=%x", *(unsigned *)(*p20 + 0x34));
    }
    fprintf(leslog, "\n");
    num_toks("T", (unsigned char *)args[4], (int)args[2]);
    fflush(leslog);
}
void __cdecl num_post(unsigned rc) {
    fprintf(leslog, "< NUM");
    les_str("ret", (unsigned char *)rc);
    les_str("out", (unsigned char *)num_args[5]);
    unsigned c = num_args[0];
    if (c && !IsBadReadPtr((void *)c, 0x20)) fprintf(leslog, " c18=%d", *(int *)(c + 0x18));
    fprintf(leslog, "\n");
    num_toks("U", (unsigned char *)num_args[4], (int)num_args[2]);
    fflush(leslog);
}
void num_thunk(void);
__asm__(".globl _num_thunk\n_num_thunk:\n"
        " pushal\n leal 36(%esp), %eax\n pushl %eax\n call _num_pre\n addl $4, %esp\n popal\n"
        " popl _num_ret\n call *_num_real\n pushl _num_ret\n"
        " pushal\n pushl %eax\n call _num_post\n addl $4, %esp\n popal\n ret\n");
static int les_quiet(void) { static int q = -1; if (q < 0) q = getenv("LOQ_NUM_ONLY") != 0; return q; }

void __cdecl les_pre(unsigned i, unsigned *args) {
    memcpy(les_args[i], args, sizeof les_args[i]);
    if (les_quiet()) return;
    fprintf(leslog, "> %s", les_name[i]);
    for (int k = 0; k < 6; k++) fprintf(leslog, " %08x", args[k]);
    const char *n = les_name[i];
    if (!strcmp(n, "FonAllofonoLingua")) {
        fprintf(leslog, " idx=%u\n", args[1]);
        if (args[1] == 0) {
            unsigned c = args[0];
            if (c && !IsBadReadPtr((void *)c, 0x58)) {
                unsigned ch = *(unsigned *)(c + 0x28) + *(unsigned *)(c + 0x38) * 0x1c;
                fprintf(leslog, "  C ctx20=%u ctx38=%u", *(unsigned *)(c + 0x20), *(unsigned *)(c + 0x38));
                if (!IsBadReadPtr((void *)ch, 0x1c)) fprintf(leslog, " ch4=%u ch18=%u ch0=%u", *(unsigned *)(ch + 4), *(unsigned *)(ch + 0x18), *(unsigned *)ch);
                fprintf(leslog, "\n");
            }
            les_phones(args[0]);
        }
        fflush(leslog);
        return;
    }
    if (!strcmp(n, "FonTrasParola")) {
        les_str("word", (unsigned char *)args[2]);
        fprintf(leslog, " idx=%u\n", args[4]);
        unsigned c = args[0];
        if (getenv("LOQ_LES_DA") && c && !IsBadReadPtr((void *)c, 0x3c)) {
            /* the record: FON ctx+0x2c records, chunk (ctx+0x28 + ctx+0x38 *
               0x1c) +0x18 its first, as 0x1000b6d0 finds it */
            unsigned ch = *(unsigned *)(c + 0x28) + *(unsigned *)(c + 0x38) * 0x1c;
            if (!IsBadReadPtr((void *)ch, 0x1c)) {
                unsigned r = *(unsigned *)(c + 0x2c) + (*(unsigned *)(ch + 0x18) + args[4]) * 0x18;
                if (!IsBadReadPtr((void *)r, 0x18)) {
                    fprintf(leslog, "  R %u", *(unsigned *)(ch + 0x18) + args[4]);
                    for (int k = 0; k < 24; k++) fprintf(leslog, "%s%02x", k % 4 ? "" : " ", ((unsigned char *)r)[k]);
                    fprintf(leslog, "\n");
                    les_da_lexinfo(*(unsigned *)(r + 0x14));
                }
            }
        }
    } else if (!strcmp(n, "FonTrasConfini")) {
        unsigned *code = (unsigned *)args[1];
        int *p3 = (int *)args[2];
        fprintf(leslog, " code=%x p3=%d p4=%d\n", code ? *code : 0, p3 ? *p3 : -1, (int)args[3]);
    } else if (!strcmp(n, "LesNormalizzaAccento")) {
        les_str("s", (unsigned char *)args[0]);
        fprintf(leslog, "\n");
    } else if (!strcmp(n, "LesPronunciabile")) {
        les_str("s", (unsigned char *)args[2]);
        fprintf(leslog, " c=%d\n", (int)(char)args[3]);
    } else {
        unsigned c = args[0];
        if (c && !IsBadReadPtr((void *)c, 32)) fprintf(leslog, " ctx=%08x,%08x,%08x,%08x,%08x", ((unsigned *)c)[0], ((unsigned *)c)[1], ((unsigned *)c)[2], ((unsigned *)c)[3], ((unsigned *)c)[4]);
        fprintf(leslog, "\n");
        les_da_on = !strcmp(n, "LesAccenta") || !strcmp(n, "LesAnalizza");
        if (!strncmp(n, "Les", 3) && strcmp(n, "LesGetSymbolName")) les_words(strcmp(n, "LesGestisciMaiuscole") ? args[0] : args[1]);
        les_da_on = 0;
    }
    fflush(leslog);
}

void __cdecl les_post(unsigned i, unsigned rc) {
    unsigned *args = les_args[i];
    const char *n = les_name[i];
    if (!strcmp(n, "LesIniNumeri") && args[0] && !num_real) {
        void **slots = (void **)args[0];
        num_real = slots[9];
        slots[9] = (void *)num_thunk;
    }
    if (les_quiet()) return;
    fprintf(leslog, "< %s rc=%x", n, rc);
    if (!strcmp(n, "FonTrasParola")) {
        unsigned *out = (unsigned *)args[1];
        fprintf(leslog, " out=");
        for (int k = 0; out && k < 256 && out[k]; k++) fprintf(leslog, "%s%x", k ? "," : "", out[k]);
        les_str("word", (unsigned char *)args[2]);
        fprintf(leslog, "\n");
    } else if (!strcmp(n, "FonTrasConfini")) {
        fprintf(leslog, "\n");
    } else if (!strcmp(n, "FonAllofonoLingua")) {
        fprintf(leslog, "\n");
        if ((int)rc + 2 >= les_nphones(args[0])) { fprintf(leslog, "  Q"); les_phones(args[0]); }
    } else if (!strcmp(n, "LesNormalizzaAccento")) {
        les_str("s", (unsigned char *)args[0]);
        fprintf(leslog, "\n");
    } else if (!strcmp(n, "LesPronunciabile")) {
        unsigned char *b = (unsigned char *)args[1];
        fprintf(leslog, " ok=%d\n", b ? *b : -1);
    } else if (!strcmp(n, "LesGetSymbolName")) {
        les_str("s", (unsigned char *)args[0]);
        fprintf(leslog, " c=%d\n", (int)(char)args[1]);
    } else {
        fprintf(leslog, "\n");
        les_da_on = !strcmp(n, "LesAccenta") || !strcmp(n, "LesAnalizza");
        if (!strncmp(n, "Les", 3)) les_words(strcmp(n, "LesGestisciMaiuscole") ? args[0] : args[1]);
        les_da_on = 0;
    }
    fflush(leslog);
}

/* thunk i: log, call the real function with the caller's own arguments
 * (cdecl: the caller cleans), log, return; ECX/EDX are passed through */
#define THUNK(i) \
    __asm__(".globl _les_thunk" #i "\n_les_thunk" #i ":\n" \
            " pushal\n leal 36(%esp), %eax\n pushl %eax\n pushl $" #i "\n call _les_pre\n addl $8, %esp\n popal\n" \
            " popl _les_ret+4*" #i "\n call *_les_real+4*" #i "\n pushl _les_ret+4*" #i "\n" \
            " pushal\n pushl %eax\n pushl $" #i "\n call _les_post\n addl $8, %esp\n popal\n ret\n"); \
    void les_thunk##i(void);
THUNK(0) THUNK(1) THUNK(2) THUNK(3) THUNK(4) THUNK(5) THUNK(6) THUNK(7) THUNK(8) THUNK(9)
THUNK(10) THUNK(11) THUNK(12) THUNK(13) THUNK(14) THUNK(15) THUNK(16) THUNK(17) THUNK(18)
THUNK(19) THUNK(20) THUNK(21) THUNK(22) THUNK(23) THUNK(24) THUNK(25) THUNK(26) THUNK(27) THUNK(28) THUNK(29)
THUNK(30) THUNK(31) THUNK(32) THUNK(33) THUNK(34) THUNK(35) THUNK(36) THUNK(37) THUNK(38) THUNK(39)
THUNK(40) THUNK(41) THUNK(42) THUNK(43) THUNK(44) THUNK(45) THUNK(46) THUNK(47) THUNK(48) THUNK(49)
THUNK(50) THUNK(51) THUNK(52) THUNK(53) THUNK(54) THUNK(55) THUNK(56) THUNK(57) THUNK(58) THUNK(59)
THUNK(60) THUNK(61) THUNK(62) THUNK(63)
static void (*les_thunks[NSLOT])(void) = {les_thunk0, les_thunk1, les_thunk2, les_thunk3, les_thunk4,
    les_thunk5, les_thunk6, les_thunk7, les_thunk8, les_thunk9, les_thunk10, les_thunk11, les_thunk12,
    les_thunk13, les_thunk14, les_thunk15, les_thunk16, les_thunk17, les_thunk18, les_thunk19, les_thunk20,
    les_thunk21, les_thunk22, les_thunk23, les_thunk24, les_thunk25, les_thunk26, les_thunk27, les_thunk28, les_thunk29,
    les_thunk30, les_thunk31, les_thunk32, les_thunk33, les_thunk34, les_thunk35, les_thunk36, les_thunk37, les_thunk38,
    les_thunk39, les_thunk40, les_thunk41, les_thunk42, les_thunk43, les_thunk44, les_thunk45, les_thunk46, les_thunk47,
    les_thunk48, les_thunk49, les_thunk50, les_thunk51, les_thunk52, les_thunk53, les_thunk54, les_thunk55, les_thunk56,
    les_thunk57, les_thunk58, les_thunk59, les_thunk60, les_thunk61, les_thunk62, les_thunk63};

static int hook_language(const char *bin, const char *dll, unsigned table, unsigned n) {
    char path[MAX_PATH];
    snprintf(path, sizeof path, "%s\\%s", bin, dll);
    HMODULE m = LoadLibraryA(path);
    if (!m) { fprintf(stderr, "cannot load %s\n", path); return -1; }
    les_base = (unsigned)m;
    unsigned *t = (unsigned *)((char *)m + (table - 0x10000000));
    DWORD old;
    VirtualProtect(t, 8 * n, PAGE_READWRITE, &old);
    for (unsigned i = 0; i < n && i < NSLOT; i++) {
        les_name[i] = (const char *)t[2 * i];
        les_real[i] = (void *)t[2 * i + 1];
        t[2 * i + 1] = (unsigned)les_thunks[i];
    }
    fprintf(leslog, "B %08x\n", les_base);
    return 0;
}
