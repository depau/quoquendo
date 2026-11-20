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
    if (chunk && n && getenv("LOQ_TRACE_RESAMPLE")[0] >= '2') {
        short *s = **(short ***)(chunk + 0x248);
        for (unsigned i = 0; i < n && s; i++) fprintf(reslog, " %d", s[i]);
    }
    fprintf(reslog, "\n");
    return real_res(state, rates, chunk, out);
}
/* LOQ_TRACE_RESAMPLE=3 also logs every call of the resampler proper
 * (0x1001c930 -> 0x1001c370 at 0x1001c9e2: resampler object in ESI, cdecl
 * int *inCount, nOut, short *in, short *out, Imp, ImpD, LpScl, Nmult, Nwing)
 * with its state before and after: Time (+0x18), the 5120-sample input
 * history X (**(+0x10)), and what it returned:
 *   C <inCount> <nOut> <Time> <first +0x2c> <LpScl> <Nmult> <Nwing> <factor>
 *   I <inCount samples>      X <5120 samples>
 *   O <outputs> <consumed> <Time>
 *   Y <outputs samples>      X <5120 samples>
 * and the end-of-utterance flush (0x10015be5 -> 0x10028e40, ECX = &state):
 *   F <pending +0x18> */
void *real_src, *real_flush;
unsigned char *src_obj;
unsigned src_ret, flush_state;
int *src_in;
short *src_out;
static void dump_x(void) {
    short *x = **(short ***)(src_obj + 0x10);
    fprintf(reslog, "X");
    for (int i = 0; i < 5120; i++) fprintf(reslog, " %d", x[i]);
    fprintf(reslog, "\n");
}
void __cdecl src_pre(int *in_count, unsigned n_out, short *in, short *out, void *imp, void *impd, unsigned lp, unsigned nmult, unsigned nwing) {
    (void)imp; (void)impd;
    src_in = in_count;
    src_out = out;
    fprintf(reslog, "C %d %u %u %u %u %u %u %.17g\nI", *in_count, n_out, *(unsigned *)(src_obj + 0x18), src_obj[0x2c], lp & 0xffff,
            nmult & 0xffff, nwing & 0xffff, *(double *)(src_obj + 0x20));
    for (int i = 0; i < *in_count; i++) fprintf(reslog, " %d", in[i]);
    fprintf(reslog, "\n");
    dump_x();
}
void __cdecl src_post(int ret) {
    fprintf(reslog, "O %d %d %u\nY", ret, *src_in, *(unsigned *)(src_obj + 0x18));
    for (int i = 0; i < ret; i++) fprintf(reslog, " %d", src_out[i]);
    fprintf(reslog, "\n");
    dump_x();
    fflush(reslog);
}
void hook_src(void);
__asm__(".globl _hook_src\n_hook_src:\n"
        " movl %esi, _src_obj\n popl _src_ret\n call _src_pre\n movl _src_obj, %esi\n call *_real_src\n"
        " pushl _src_ret\n pushl %eax\n pushl %eax\n call _src_post\n addl $4, %esp\n popl %eax\n ret\n");
void __cdecl flush_pre(void) {
    unsigned char *st = *(unsigned char **)flush_state;
    fprintf(reslog, "F %d\n", st ? *(int *)(st + 0x18) : -1);
    fflush(reslog);
}
void hook_flush(void);
__asm__(".globl _hook_flush\n_hook_flush:\n"
        " movl %ecx, _flush_state\n pushl %ecx\n call _flush_pre\n popl %ecx\n jmp *_real_flush\n");
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

/* LOQ_TRACE_TEXT=1: the engine's text rewriting (LesRun, 0x10035b20) to
 * <prefix>.txt. Every ELQTextFilterApply call (LTTS7Util.dll, through
 * LoqTTS7.dll's pointer at 0x100b7690: filter, flag, ?, token list,
 * pattern array, language) logs the raw word list before (`<`) and after
 * (`>`) with LTTS7Util's own ELQRawWordListDump (word, separator, CRC,
 * offset, length, tag, type, end flags), and the text LesRun hands the
 * sentence loop (0x10035cf8 -> 0x10034110, the text in ECX) on a `T` line. */
FILE *txtlog;
typedef int (__cdecl *fn_tfa)(void *, int, void *, void *, void *, void *);
typedef void (__cdecl *fn_rwdump)(void *, void *, void *, int, unsigned, char);
static fn_tfa real_tfa;
static fn_rwdump rwdump;
static void __cdecl txt_line(const char *s, void *f) { fputs(s, (FILE *)f); }
static int __cdecl hook_tfa(void *filter, int flag, void *a3, void *list, void *pm, void *lang) {
    fprintf(txtlog, "< filter %p flag %d\n", filter, flag);
    rwdump(list, (void *)txt_line, txtlog, 0, 0, 0);
    int rc = real_tfa(filter, flag, a3, list, pm, lang);
    fprintf(txtlog, "> rc %d\n", rc);
    rwdump(list, (void *)txt_line, txtlog, 0, 0, 0);
    fflush(txtlog);
    return rc;
}
void *real_text;
void __cdecl log_text(const char *s) { fprintf(txtlog, "T %s\n", s ? s : "(null)"); fflush(txtlog); }
void hook_text(void);
__asm__(".globl _hook_text\n_hook_text:\n"
        " pushal\n pushl %ecx\n call _log_text\n addl $4, %esp\n popal\n jmp *_real_text\n");

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

/* LOQ_TRACE_NUM=1: LoqTTS7.dll's generic number reader (LesNumero2Testo
 * 0x1003ebf0, the modules without a LesIniNumeri) to <prefix>.num. Two of
 * its LTTS7Util pointers are swapped: ELQNumSetContextChunkPointer
 * (0x100b7180, called once per number with the token buffer, 0x40c bytes a
 * token, type at +0x400) logs the focus and the tokens up to the end type 7,
 *   I <focus> <k>:<type>:"text" …
 * and ELQNumDelete (0x100b71e0, after the reading) the ELQNum object (0x524
 * bytes: type +4, subtype +8, escape +0xc, reading mode +0x110, skip +0x21c,
 * ints +0x220…+0x234, strings +0x220 +0x228 +0x22c +0x320 +0x32c +0x420)
 * and its morph list (+0x210: {tag, ids, cap, count}):
 *   N type=… sub=… esc=… skip=… mode="…" i=… s220=… s228=… s22c=… s320=… s32c=… s420=… m=<ids> */
FILE *numlog;
typedef int (__cdecl *fn_num1)(void *);
typedef int (__cdecl *fn_num2)(void *, void *);
static fn_num1 real_numdelete;
static fn_num2 real_numchunk;
static void num_str(const char *tag, const unsigned char *s) {
    FILE *save = leslog;
    leslog = numlog;
    les_str(tag, s);
    leslog = save;
}
static int __cdecl hook_numchunk(void *num, void *toks) {
    unsigned focus = *(unsigned *)((char *)num + 0x214);
    fprintf(numlog, "I %u", focus);
    for (unsigned k = 0; k < 32 && toks; k++) {
        unsigned char *t = (unsigned char *)toks + k * 0x40c;
        if (IsBadReadPtr(t, 0x40c)) break;
        unsigned type = *(unsigned *)(t + 0x400);
        char tag[16];
        snprintf(tag, sizeof tag, "%u:%u:", k, type);
        fprintf(numlog, " %s", tag);
        num_str("", t);
        if (type == 7) break;
    }
    fprintf(numlog, "\n");
    return real_numchunk(num, toks);
}
static int __cdecl hook_numdelete(void *num) {
    unsigned char *o = (unsigned char *)num;
    if (o && !IsBadReadPtr(o, 0x524)) {
        fprintf(numlog, "N type=%d sub=%d esc=%d skip=%d", *(int *)(o + 4), *(int *)(o + 8), *(int *)(o + 0xc), *(int *)(o + 0x21c));
        num_str("mode", o + 0x110);
        fprintf(numlog, " i=%d,%d,%d,%d,%d,%d", *(int *)(o + 0x220), *(int *)(o + 0x224), *(int *)(o + 0x228), *(int *)(o + 0x22c),
                *(int *)(o + 0x230), *(int *)(o + 0x234));
        num_str("s220", o + 0x220);
        num_str("s228", o + 0x228);
        num_str("s22c", o + 0x22c);
        num_str("s320", o + 0x320);
        num_str("s32c", o + 0x32c);
        num_str("s420", o + 0x420);
        unsigned *list = *(unsigned **)(o + 0x210);
        fprintf(numlog, " m=");
        if (list && !IsBadReadPtr(list, 0x14) && list[1] && !IsBadReadPtr((void *)list[1], 4 * list[3]))
            for (unsigned k = 0; k < list[3]; k++) fprintf(numlog, k ? ",%d" : "%d", ((int *)list[1])[k]);
        fprintf(numlog, "\n");
        fflush(numlog);
    }
    return real_numdelete(num);
}

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
        if (getenv("LOQ_TRACE_RESAMPLE")[0] == '3') {
            real_src = redirect(dll, 0x1001c9e2, (void *)hook_src);
            real_flush = redirect(dll, 0x10015be5, (void *)hook_flush);
        }
    }
    if (getenv("LOQ_TRACE_NUM")) {
        snprintf(path, sizeof path, "%s.num", argv[4]);
        numlog = fopen(path, "wb");
        void **chunk = (void **)((char *)dll + (0x100b7180 - 0x10000000));
        void **del = (void **)((char *)dll + (0x100b71e0 - 0x10000000));
        if (!*chunk || !*del) { fprintf(stderr, "ELQNum pointers not set yet\n"); return 2; }
        DWORD old;
        VirtualProtect(chunk, 0x64, PAGE_READWRITE, &old);
        real_numchunk = (fn_num2)*chunk;
        *chunk = (void *)hook_numchunk;
        real_numdelete = (fn_num1)*del;
        *del = (void *)hook_numdelete;
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
                          getenv("LOQ_LES_TABLE") ? strtoul(getenv("LOQ_LES_TABLE"), 0, 16) : 0x10031ff8,
                          getenv("LOQ_LES_N") ? atoi(getenv("LOQ_LES_N")) : 19) < 0) return 2;
    }
    if (getenv("LOQ_TRACE_TEXT")) {
        snprintf(path, sizeof path, "%s.txt", argv[4]);
        txtlog = fopen(path, "wb");
        snprintf(path, sizeof path, "%s\\LTTS7Util.dll", argv[1]);
        HMODULE util = LoadLibraryA(path);
        rwdump = util ? (fn_rwdump)GetProcAddress(util, "ELQRawWordListDump") : NULL;
        fn_tfa *slot = (fn_tfa *)((char *)dll + (0x100b7690 - 0x10000000));
        if (!rwdump || !*slot) { fprintf(stderr, "ELQTextFilterApply/ELQRawWordListDump not found\n"); return 2; }
        DWORD old;
        VirtualProtect(slot, 4, PAGE_READWRITE, &old);
        real_tfa = *slot;
        *slot = hook_tfa;
        real_text = redirect(dll, 0x10035cf8, (void *)hook_text);
        /* name the Util functions LesRun calls through LoqTTS7.dll's pointers */
        unsigned char *ub = (unsigned char *)util;
        IMAGE_EXPORT_DIRECTORY *ed = (IMAGE_EXPORT_DIRECTORY *)(ub + ((IMAGE_NT_HEADERS *)(ub + ((IMAGE_DOS_HEADER *)ub)->e_lfanew))
                                         ->OptionalHeader.DataDirectory[0].VirtualAddress);
        unsigned slots[] = {0x100b75fc, 0x100b7600, 0x100b7604, 0x100b7608, 0x100b760c, 0x100b770c, 0x100b7590, 0x100b758c};
        for (unsigned s = 0; s < sizeof slots / sizeof *slots; s++) {
            void *fn = *(void **)((char *)dll + (slots[s] - 0x10000000));
            const char *nm = "?";
            for (DWORD k = 0; k < ed->NumberOfNames; k++) {
                WORD o = ((WORD *)(ub + ed->AddressOfNameOrdinals))[k];
                if (ub + ((DWORD *)(ub + ed->AddressOfFunctions))[o] == fn) nm = (const char *)ub + ((DWORD *)(ub + ed->AddressOfNames))[k];
            }
            fprintf(txtlog, "# %08x %s\n", slots[s], nm);
        }
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

        /* the number trace: the transcription's numbers, then the read's */
        if (numlog) { fprintf(numlog, "U %d\n", i); fflush(numlog); }
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
