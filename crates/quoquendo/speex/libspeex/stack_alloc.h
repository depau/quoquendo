/* quoquendo: minimal stack allocation macros for the Speex sources.
   With VAR_ARRAYS scratch buffers are C99 variable length arrays; with
   USE_ALLOCA they come from alloca(). The manual "stack" pointer the
   sources pass around is then unused. */
#ifndef STACK_ALLOC_H
#define STACK_ALLOC_H

#ifdef USE_ALLOCA
# ifdef _WIN32
#  include <malloc.h>
# else
#  include <alloca.h>
# endif
#endif

#define ALIGN(stack, size) ((stack) += ((size) - (long)(stack)) & ((size) - 1))
#define PUSH(stack, size, type) (ALIGN((stack),sizeof(type)/sizeof(char)),(stack)+=(size)*(sizeof(type)/sizeof(char)),(type*)((stack)-(size)*(sizeof(type)/sizeof(char))))

#if defined(VAR_ARRAYS)
# define VARDECL(var)
# define ALLOC(var, size, type) type var[size]
#elif defined(USE_ALLOCA)
# define VARDECL(var) var
# define ALLOC(var, size, type) var = alloca(sizeof(type)*(size))
#else
# error "define VAR_ARRAYS or USE_ALLOCA"
#endif

#endif
