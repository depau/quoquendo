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
 * Up to 19 slots (NSLOT). */
FILE *leslog;
#define NSLOT 19
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
    }
}

void __cdecl les_pre(unsigned i, unsigned *args) {
    memcpy(les_args[i], args, sizeof les_args[i]);
    fprintf(leslog, "> %s", les_name[i]);
    for (int k = 0; k < 6; k++) fprintf(leslog, " %08x", args[k]);
    const char *n = les_name[i];
    if (!strcmp(n, "FonTrasParola")) {
        les_str("word", (unsigned char *)args[2]);
        fprintf(leslog, " idx=%u\n", args[4]);
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
        if (!strncmp(n, "Les", 3) && strcmp(n, "LesGetSymbolName")) les_words(args[0]);
    }
    fflush(leslog);
}

void __cdecl les_post(unsigned i, unsigned rc) {
    unsigned *args = les_args[i];
    const char *n = les_name[i];
    fprintf(leslog, "< %s rc=%x", n, rc);
    if (!strcmp(n, "FonTrasParola")) {
        unsigned *out = (unsigned *)args[1];
        fprintf(leslog, " out=");
        for (int k = 0; out && k < 256 && out[k]; k++) fprintf(leslog, "%s%x", k ? "," : "", out[k]);
        les_str("word", (unsigned char *)args[2]);
        fprintf(leslog, "\n");
    } else if (!strcmp(n, "FonTrasConfini")) {
        fprintf(leslog, "\n");
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
        if (!strncmp(n, "Les", 3)) les_words(args[0]);
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
static void (*les_thunks[NSLOT])(void) = {les_thunk0, les_thunk1, les_thunk2, les_thunk3, les_thunk4,
    les_thunk5, les_thunk6, les_thunk7, les_thunk8, les_thunk9, les_thunk10, les_thunk11, les_thunk12,
    les_thunk13, les_thunk14, les_thunk15, les_thunk16, les_thunk17, les_thunk18};

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
