/* loqref: run the original Loquendo 7 engine on one text and record what each
 * of its stages produced, so quoquendo can be compared against it.
 *
 *   loqref.exe <LTTS7 bin folder> <voice> <utf-8 text file> <output prefix>
 *
 * writes <prefix>.fon   the engine's phonetic transcription (ttsPhoneticTranscription)
 *        <prefix>.ev    one line per engine event (phonemes, unit selection, ...)
 *        <prefix>.wav   the engine's audio (LTTS7AudioFile device)
 *
 * The engine needs its licence; oracle/run.sh provides it. Build:
 *   i686-w64-mingw32-gcc -O1 -o loqref.exe loqref.c
 */
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

typedef void *H;
typedef int (__stdcall *fn_newsession)(H *, const char *);
typedef int (__stdcall *fn_newreader)(H *, H);
typedef int (__stdcall *fn_loadpersona)(H, const char *, const char *, const char *);
typedef int (__stdcall *fn_settextenc)(H, int);
typedef int (__stdcall *fn_phonetic)(H, const void *, char *, unsigned, unsigned *);
typedef int (__stdcall *fn_enable)(H, int, unsigned char);
typedef int (__cdecl *fn_cb)(unsigned, int, void *, void *);
typedef int (__stdcall *fn_setcb)(H, fn_cb, void *, int);
typedef int (__stdcall *fn_setaudio)(H, const char *, const char *, unsigned, int, int, const void *);
typedef int (__stdcall *fn_read)(H, const void *, unsigned char, unsigned char, unsigned *);
typedef int (__stdcall *fn_phinfo)(void *, char **, unsigned *);
typedef int (__stdcall *fn_setparam)(H, const char *, const char *);
typedef const char *(__stdcall *fn_errmsg)(int);

static FILE *ev;
#include "lestrace.h"

/* LOQ_DUMP_TABLES=<prefix>: catch the CAT module's phonetic tables as the
 * voice loads. LoqTTS7.dll stores them with ELQSharedResourceAdd (its pointer
 * at 0x100b7324, cdecl, 7 arguments: manager, &object, name, 0, 0, 0, voice);
 * the constructors are 0x100498f0 (TabelleFonetiche_: name, uint32 features[N],
 * uint8 class[N]) and 0x10049a70 (SomiglianzaFonetica_: name, uint8 sim[N*N]),
 * N = *(*(voice+0xc)+4), phone entries 16 bytes at **(voice+0xc). */
/* LOQ_TRACE_CAT=1: log every candidate the picker scores to <prefix>.cat, by
 * redirecting LoqTTS7.dll's calls in 0x10046b20 to the context score
 * (0x10046c9d -> 0x10045ad0), the factor (0x10046d2f -> 0x10046670) and the
 * prosody term (0x10046d65 -> 0x100468c0, position in EDX, caller cleans):
 *   C <half k> <unit first phone> <pos> <score> <class part>
 *   F <half k> <unit first phone> <pos> <factor> <halves taken>
 *   P <pos> <prosody>                (belongs to the C line before it) */
FILE *catlog;
typedef int (__cdecl *fn6)(void *, void *, void *, int, int *, int);
static fn6 real_ctx, real_factor;
void *real_pros;
unsigned pros_edx, pros_ret;
static int __cdecl hook_ctx(void *st, void *ph, void *tg, int mood, int *unit, int pos) {
    int r = real_ctx(st, ph, tg, mood, unit, pos);
    unsigned short k = *(unsigned short *)((char *)st + 0x170);
    fprintf(catlog, "C %u %d %d %d %d", k, unit[0], pos, r, *(int *)((char *)st + 0x3c4));
    if (getenv("LOQ_TRACE_CLASSES")) {
        /* the recording's phone ids and prosodic classes, and the wanted
           phrase's classes (state+0x4d0) */
        unsigned short n = *(unsigned short *)((char *)unit + 4);
        fprintf(catlog, " U");
        for (int j = 0; j < n; j++) fprintf(catlog, " %d:%d", ((unsigned char *)ph)[2 * j], ((unsigned char *)ph)[2 * j + 1]);
        fprintf(catlog, " T");
        for (int j = 0; j < 40; j++) fprintf(catlog, " %d", ((signed char *)st)[0x4d0 + j]);
    }
    fprintf(catlog, "\n");
    return r;
}
static int __cdecl hook_factor(void *st, void *ph, void *tg, int halves, int *unit, int pos) {
    int r = real_factor(st, ph, tg, halves, unit, pos);
    fprintf(catlog, "F %u %d %d %d %d\n", *(unsigned short *)((char *)st + 0x170), unit[0], pos, r, *(short *)((char *)halves + 8));
    return r;
}
void __cdecl log_pros(int v) { fprintf(catlog, "P %u %d\n", pros_edx & 0xffff, v); }
void hook_pros(void);
__asm__(".globl _hook_pros\n_hook_pros:\n"
        " movl %edx, _pros_edx\n popl _pros_ret\n call *_real_pros\n"
        " pushl _pros_ret\n pushl %eax\n pushl %eax\n call _log_pros\n"
        " addl $4, %esp\n popl %eax\n ret\n");
/* LOQ_TRACE_JOIN=1: log every join of two runs (0x1001f8a3 -> 0x10024660,
 * the current unit in EDX, the previous one on the stack, caller cleans) to
 * <prefix>.join: each unit's start/pre/len/post/voiced, pitch marks (the
 * array at +0x1c, count at +0x41c), and the same after the join, plus a
 * checksum of its samples. LOQ_TRACE_JOIN=2 adds every sample (pre + len +
 * post) on an `S` line after each unit. */
FILE *joinlog;
void *real_join;
unsigned join_a, join_b, join_ret;
static void log_unit(const char *tag, unsigned char *u) {
    short *buf = **(short ***)(u + 8);
    unsigned len = *(unsigned *)(u + 0xc), post = *(unsigned *)(u + 0x10), pre = *(unsigned *)(u + 0x14);
    unsigned nm = *(unsigned *)(u + 0x41c);
    long long sum = 0;
    for (unsigned i = 0; i < pre + len + post && buf; i++) sum += (long long)buf[i] * (long long)(i + 1);
    fprintf(joinlog, "%s start=%u pre=%u len=%u post=%u voiced=%u base=%d sum=%lld marks", tag, *(unsigned *)(u + 0x18), pre, len, post,
            u[0x640], *(int *)(u + 0x1c), sum);
    for (unsigned i = 0; i < nm && i < 256; i++) fprintf(joinlog, " %d", *(int *)(u + 0x1c + 4 * i));
    fprintf(joinlog, "\n");
    if (buf && getenv("LOQ_TRACE_JOIN")[0] == '2') {
        fprintf(joinlog, "S");
        for (unsigned i = 0; i < pre + len + post; i++) fprintf(joinlog, " %d", buf[i]);
        fprintf(joinlog, "\n");
    }
}
void __cdecl join_pre(void) { log_unit("A", (unsigned char *)join_a); log_unit("B", (unsigned char *)join_b); }
void __cdecl join_post(void) { log_unit("A'", (unsigned char *)join_a); log_unit("B'", (unsigned char *)join_b); fflush(joinlog); }
/* and every unit's pitch marks (0x1001fbd1 -> 0x1001e3a0, cdecl engine,
 * unit info): its ticks, F0 (+0x140), voicing (+0x150), algorithm (+0xc: 0
 * inside a run, 1 at its end) and the marks. LOQ_TRACE_UNITS=1 logs the same
 * M lines alone to <prefix>.units: every half-phone SIG plays, in order. */
FILE *unitlog;
typedef void (__cdecl *fn_marks)(unsigned char *, unsigned char *);
static fn_marks real_marks;
static void __cdecl hook_marks(unsigned char *e, unsigned char *info) {
    real_marks(e, info);
    unsigned n = *(unsigned *)(e + 0x1528);
    FILE *logs[] = {joinlog, unitlog};
    for (int k = 0; k < 2; k++) {
        FILE *f = logs[k];
        if (!f) continue;
        fprintf(f, "M t0=%u t1=%u f0=%u voc=%c algo=%u margin=%u marks", *(unsigned *)(e + 0x155c), *(unsigned *)(e + 0x1564),
                *(unsigned *)(info + 0x140), info[0x150] ? info[0x150] : '-', *(unsigned *)(info + 0xc), *(unsigned *)(e + 0x25c));
        for (unsigned i = 0; i < n && i < 256; i++) fprintf(f, " %u", *(unsigned *)(e + 0x1128 + 4 * i));
        fprintf(f, "\n");
    }
}
void hook_join(void);
__asm__(".globl _hook_join\n_hook_join:\n"
        " movl %edx, _join_b\n movl 4(%esp), %eax\n movl %eax, _join_a\n"
        " pushl %ecx\n pushl %edx\n call _join_pre\n popl %edx\n popl %ecx\n"
        " popl _join_ret\n call *_real_join\n pushl _join_ret\n"
        " pushl %eax\n call _join_post\n popl %eax\n ret\n");

/* LOQ_TRACE_RESAMPLE=1: log every chunk SIG hands the output resampler
 * (VoiceResamplerRun 0x10028ee0, called once at 0x10015c77, cdecl: state,
 * rates, chunk, output) to <prefix>.res: its sample count (+4), and with
 * LOQ_TRACE_RESAMPLE=2 its samples (handle at +0x248):
 *   R <count> [samples...] */
FILE *reslog;
typedef int (__cdecl *fn_res)(int *, unsigned *, unsigned char *, void *);
static fn_res real_res;
static int __cdecl hook_res(int *state, unsigned *rates, unsigned char *chunk, void *out) {
    unsigned n = chunk ? *(unsigned *)(chunk + 4) : 0;
    fprintf(reslog, "R %u", n);
    if (chunk && n && getenv("LOQ_TRACE_RESAMPLE")[0] == '2') {
        short *s = **(short ***)(chunk + 0x248);
        for (unsigned i = 0; i < n && s; i++) fprintf(reslog, " %d", s[i]);
    }
    fprintf(reslog, "\n");
    return real_res(state, rates, chunk, out);
}
static void *redirect(HMODULE dll, unsigned site, void *to) {
    unsigned char *at = (unsigned char *)dll + (site - 0x10000000);
    DWORD old;
    VirtualProtect(at, 5, PAGE_EXECUTE_READWRITE, &old);
    void *was = at + 5 + *(int *)(at + 1);
    *(int *)(at + 1) = (int)((unsigned char *)to - (at + 5));
    return was;
}

/* LOQ_TRACE_DECODE=1: log every decoder request to <prefix>.dec. The codecs
 * hand LoqTTS7.dll their functions through a name table (LTTS7Codec210.dll
 * 0x1002e028, LTTS7CodecMsx.dll 0x10015ae0); the ELQDecode slot is swapped
 * for a wrapper before the voice loads. ELQDecode (Codec210 0x10004530, Msx
 * 0x10002780) is cdecl (decoder, position, samples, short *out, file) and
 * returns 0; the decoder adds its delay to the position itself. One line per
 * request, with the x87 control word and MXCSR the decode ran under:
 *   D <codec> <decoder> <position> <samples> <rc> <x87 control word> <mxcsr> <samples...>
 * `U <n>` before each utterance, and per codec loaded its C runtime's SSE2
 * flags (exp/floor take their SSE2 routines when the first is set, _ftol2
 * when the second is: Codec210 0x100300cc/0x100300d0, Msx 0x1001c184/
 * 0x1001c18c):
 *   S <codec> <flag> <flag> */
FILE *declog;
typedef int (__cdecl *fn_dec)(void *, unsigned, unsigned, short *, void *);
static fn_dec real_dec210, real_decmsx;
static int log_dec(fn_dec real, const char *codec, void *ctx, unsigned pos, unsigned n, short *out, void *file) {
    unsigned short cw;
    unsigned mxcsr;
    __asm__ volatile("fnstcw %0\n stmxcsr %1" : "=m"(cw), "=m"(mxcsr));
    int rc = real(ctx, pos, n, out, file);
    fprintf(declog, "D %s %p %u %u %d %04x %04x", codec, ctx, pos, n, rc, cw, mxcsr);
    for (unsigned i = 0; i < n && out; i++) fprintf(declog, " %d", out[i]);
    fprintf(declog, "\n");
    return rc;
}
static int __cdecl hook_dec210(void *ctx, unsigned pos, unsigned n, short *out, void *file) {
    return log_dec(real_dec210, "Codec210", ctx, pos, n, out, file);
}
static int __cdecl hook_decmsx(void *ctx, unsigned pos, unsigned n, short *out, void *file) {
    return log_dec(real_decmsx, "CodecMsx", ctx, pos, n, out, file);
}
/* swap the function pointer at `slot` (expected to hold `fn`) in codec `name` */
static int hook_codec(const char *bin, const char *name, unsigned slot, unsigned fn, void *to, fn_dec *real, unsigned sse2, unsigned ftol) {
    char path[MAX_PATH];
    snprintf(path, sizeof path, "%s\\%s", bin, name);
    HMODULE m = LoadLibraryA(path);
    if (!m) return 0; /* not installed: no voice of this codec */
    void **at = (void **)((char *)m + (slot - 0x10000000));
    if ((char *)*at != (char *)m + (fn - 0x10000000)) { fprintf(stderr, "%s: ELQDecode slot not where expected\n", name); return -1; }
    DWORD old;
    VirtualProtect(at, 4, PAGE_READWRITE, &old);
    *real = (fn_dec)*at;
    *at = to;
    fprintf(declog, "S %s %d %d\n", name, *(int *)((char *)m + (sse2 - 0x10000000)), *(int *)((char *)m + (ftol - 0x10000000)));
    return 1;
}

typedef int (__cdecl *fn_add)(void *, void **, const char *, int, int, int, void *);
static fn_add real_add;
static const char *dump_prefix;
static int __cdecl hooked_add(void *m, void **out, const char *name, int a, int b, int c, void *voice) {
    int rc = real_add(m, out, name, a, b, c, voice);
    if (rc || !name || !voice || !out || !*out) return rc;
    unsigned char *tab = *(unsigned char **)((char *)voice + 0xc);
    unsigned n = *(unsigned *)(tab + 4);
    unsigned char *entries = *(unsigned char **)tab;
    char path[MAX_PATH];
    if (!strncmp(name, "SomiglianzaFonetica_", 20)) {
        snprintf(path, sizeof path, "%s.sim", dump_prefix);
        FILE *f = fopen(path, "wb");
        fwrite(((unsigned char **)*out)[1], 1, n * n, f);
        fclose(f);
    } else if (!strncmp(name, "TabelleFonetiche_", 17)) {
        snprintf(path, sizeof path, "%s.phones", dump_prefix);
        FILE *f = fopen(path, "wb");
        unsigned *feat = ((unsigned **)*out)[1];
        unsigned char *cls = ((unsigned char **)*out)[2];
        for (unsigned i = 0; i < n; i++) {
            unsigned char *e = entries + 16 * i;
            char *nm = *(char **)e;
            fprintf(f, "%u\t%s\t%08x\t%u\t%u\t%u\t%08x\n", i,
                    nm && !IsBadStringPtrA(nm, 64) ? nm : "?", feat[i], cls[i], e[4], e[5], *(unsigned *)(e + 12));
        }
        fclose(f);
    }
    return rc;
}
static fn_phinfo phinfo;

static void dump(void *p, int n) {
    if (!p || IsBadReadPtr(p, n)) { fprintf(ev, " -"); return; }
    fprintf(ev, " hex=");
    for (int i = 0; i < n; i++) fprintf(ev, "%02x", ((unsigned char *)p)[i]);
}

static int __cdecl callback(unsigned prompt, int type, void *data, void *user) {
    (void)prompt; (void)user;
    if (!ev || type == 4 || type == 13 || type == 11) return 0; /* audio data / free space */
    fprintf(ev, "%d", type);
    if (type == 2 || type == 3) {
        char *name = NULL; unsigned v = 0;
        if (phinfo(data, &name, &v) == 0 && name) fprintf(ev, " phone=%s value=%u", name, v);
        dump(data, 64);
    } else {
        dump(data, type == 19 ? 192 : 96);
        if (data && !IsBadReadPtr(data, 4)) {
            char *s = *(char **)data;
            if (s && !IsBadStringPtrA(s, 256)) fprintf(ev, " str0=%.200s", s);
        }
        if (data && !IsBadStringPtrA((char *)data, 256)) fprintf(ev, " str=%.200s", (char *)data);
    }
    fprintf(ev, "\n");
    fflush(ev);
    return 0;
}

#define GET(t, n) t n = (t)GetProcAddress(dll, #n); if (!n) { fprintf(stderr, "missing %s\n", #n); return 2; }
#define CHECK(x) do { int rc_ = (x); if (rc_) { fprintf(stderr, "%s failed: %d %s\n", #x, rc_, ttsGetErrorMessage(rc_)); return 3; } } while (0)

int main(int argc, char **argv) {
    if (argc < 5) {
        fprintf(stderr, "usage: loqref <LTTS7 bin folder> <voice> <utf-8 text file> <output prefix>\n");
        return 1;
    }
    char path[MAX_PATH];
    SetDllDirectoryA(argv[1]);
    snprintf(path, sizeof path, "%s\\LoqTTS7.dll", argv[1]);
    HMODULE dll = LoadLibraryA(path);
    if (!dll) { fprintf(stderr, "cannot load %s (%lu)\n", path, GetLastError()); return 2; }
    GET(fn_newsession, ttsNewSession) GET(fn_newreader, ttsNewReader)
    GET(fn_loadpersona, ttsLoadPersona) GET(fn_settextenc, ttsSetTextEncoding)
    GET(fn_phonetic, ttsPhoneticTranscription) GET(fn_enable, ttsEnableEvent)
    GET(fn_setcb, ttsSetCallback) GET(fn_setaudio, ttsSetAudio) GET(fn_read, ttsRead)
    GET(fn_phinfo, ttsGetPhonemeInfo) GET(fn_errmsg, ttsGetErrorMessage)
    GET(fn_setparam, ttsSetParam)
    phinfo = ttsGetPhonemeInfo;

    FILE *tf = fopen(argv[3], "rb");
    if (!tf) { fprintf(stderr, "cannot read %s\n", argv[3]); return 1; }
    /* the whole file, however long (word lists run to tens of MB) */
    fseek(tf, 0, SEEK_END);
    long size = ftell(tf);
    fseek(tf, 0, SEEK_SET);
    char *all = malloc(size + 1);
    if (!all) { fprintf(stderr, "out of memory\n"); return 1; }
    size_t n = fread(all, 1, size, tf);
    all[n] = 0;
    fclose(tf);

    H session, reader;
    CHECK(ttsNewSession(&session, NULL));
    CHECK(ttsNewReader(&reader, session));
    if ((dump_prefix = getenv("LOQ_DUMP_TABLES"))) {
        fn_add *slot = (fn_add *)((char *)dll + (0x100b7324 - 0x10000000));
        DWORD old;
        VirtualProtect(slot, 4, PAGE_READWRITE, &old);
        if (!*slot) { fprintf(stderr, "ELQSharedResourceAdd pointer not set yet\n"); return 2; }
        real_add = *slot;
        *slot = hooked_add;
    }
    if (getenv("LOQ_TRACE_CAT")) {
        snprintf(path, sizeof path, "%s.cat", argv[4]);
        catlog = fopen(path, "wb");
        real_ctx = (fn6)redirect(dll, 0x10046c9d, (void *)hook_ctx);
        real_factor = (fn6)redirect(dll, 0x10046d2f, (void *)hook_factor);
        real_pros = redirect(dll, 0x10046d65, (void *)hook_pros);
    }
    if (getenv("LOQ_TRACE_JOIN")) {
        snprintf(path, sizeof path, "%s.join", argv[4]);
        joinlog = fopen(path, "wb");
        real_join = redirect(dll, 0x1001f8a3, (void *)hook_join);
    }
    if (getenv("LOQ_TRACE_UNITS")) {
        snprintf(path, sizeof path, "%s.units", argv[4]);
        unitlog = fopen(path, "wb");
    }
    if (joinlog || unitlog) real_marks = (fn_marks)redirect(dll, 0x1001fbd1, (void *)hook_marks);
    if (getenv("LOQ_TRACE_RESAMPLE")) {
        snprintf(path, sizeof path, "%s.res", argv[4]);
        reslog = fopen(path, "wb");
        real_res = (fn_res)redirect(dll, 0x10015c77, (void *)hook_res);
    }
    if (getenv("LOQ_TRACE_DECODE")) {
        snprintf(path, sizeof path, "%s.dec", argv[4]);
        declog = fopen(path, "wb");
        if (hook_codec(argv[1], "LTTS7Codec210.dll", 0x1002e064, 0x10004530, (void *)hook_dec210, &real_dec210, 0x100300cc, 0x100300d0) < 0 ||
            hook_codec(argv[1], "LTTS7CodecMsx.dll", 0x10015b1c, 0x10002780, (void *)hook_decmsx, &real_decmsx, 0x1001c184, 0x1001c18c) < 0)
            return 2;
    }
    if (getenv("LOQ_TRACE_LES")) {
        snprintf(path, sizeof path, "%s.les", argv[4]);
        leslog = fopen(path, "wb");
        if (hook_language(argv[1], getenv("LOQ_LES_DLL") ? getenv("LOQ_LES_DLL") : "LTTS7Italian.dll",
                          getenv("LOQ_LES_TABLE") ? strtoul(getenv("LOQ_LES_TABLE"), 0, 16) : 0x10031ff8, NSLOT) < 0) return 2;
    }
    CHECK(ttsLoadPersona(reader, argv[2], NULL, NULL));
    CHECK(ttsSetTextEncoding(reader, 65001));
    ttsSetParam(reader, "TaggedText", "FALSE");
    /* the voice files' own alphabet (`tt`, `T$`, `` `a ``), not X-SAMPA */
    ttsSetParam(reader, "PhoneticCoding", getenv("LOQ_CODING") ? getenv("LOQ_CODING") : "Loquendo");
    int events[] = {2, 3, 5, 6, 7, 9, 17, 19, 20, 113};
    for (unsigned i = 0; i < sizeof events / sizeof *events; i++) ttsEnableEvent(reader, events[i], 1);
    CHECK(ttsSetCallback(reader, callback, NULL, 0));

    /* LOQ_FON_ONLY=1: transcribe every line, write "line<TAB>transcription"
       lines to <prefix>.fonlist and skip the audio (fast: a word list) */
    if (getenv("LOQ_FON_ONLY")) {
        snprintf(path, sizeof path, "%s.fonlist", argv[4]);
        FILE *fl = fopen(path, "wb");
        for (char *line = strtok(all, "\r\n"); line; line = strtok(NULL, "\r\n")) {
            static char fon[16384];
            unsigned fonlen = 0;
            memset(fon, 0, sizeof fon);
            if (ttsPhoneticTranscription(reader, line, fon, sizeof fon, &fonlen) == 0)
                { fprintf(fl, "%s\t%s\n", line, fon); fflush(fl); }
        }
        fclose(fl);
        printf("ok\n");
        return 0;
    }

    /* LOQ_LINES=1: every line is its own utterance, written to <prefix>-<n>.* */
    int lines = getenv("LOQ_LINES") != NULL;
    char *text = all, *next;
    for (int i = 0; text && *text; i++, text = next) {
        next = NULL;
        if (lines) {
            char *nl = strchr(text, '\n');
            if (nl) { *nl = 0; next = nl + 1; if (nl > text && nl[-1] == '\r') nl[-1] = 0; }
        }
        char pre[MAX_PATH];
        if (lines) snprintf(pre, sizeof pre, "%s-%d", argv[4], i);
        else snprintf(pre, sizeof pre, "%s", argv[4]);
        if (!*text) continue;

        /* stage FON: the phonetic transcription only */
        static char fon[65536];
        unsigned fonlen = 0;
        memset(fon, 0, sizeof fon);
        CHECK(ttsPhoneticTranscription(reader, text, fon, sizeof fon, &fonlen));
        snprintf(path, sizeof path, "%s.fon", pre);
        FILE *ff = fopen(path, "wb");
        fwrite(fon, 1, strnlen(fon, sizeof fon), ff);
        fclose(ff);

        /* the whole pipeline, with every stage's event on */
        snprintf(path, sizeof path, "%s.ev", pre);
        ev = fopen(path, "wb");
        snprintf(path, sizeof path, "%s.wav", pre);
        CHECK(ttsSetAudio(reader, "LTTS7AudioFile", path, 32000, 0, 1, NULL));
        unsigned prompt = 0;
        if (declog) { fprintf(declog, "U %d\n", i); fflush(declog); }
        if (joinlog) { fprintf(joinlog, "U %d\n", i); fflush(joinlog); }
        if (unitlog) { fprintf(unitlog, "U %d\n", i); fflush(unitlog); }
        if (reslog) { fprintf(reslog, "U %d\n", i); fflush(reslog); }
        if (catlog) { fprintf(catlog, "U %d\n", i); fflush(catlog); }
        CHECK(ttsRead(reader, text, 0, 0, &prompt));
        if (declog) fflush(declog);
        fclose(ev);
        ev = NULL;
    }
    printf("ok\n");
    return 0;
}
