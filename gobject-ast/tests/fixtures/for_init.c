void test(void)
{
    int i;

    /* expression initializer */
    for (i = 0; i < 10; i++) {}

    /* C99 declaration initializer */
    for (int j = 0; j < 10; j++) {}

    /* pointer declaration initializer */
    GList *list = NULL;
    for (GList *l = list; l != NULL; l = l->next) {}

    /* no initializer */
    for (; i < 20; i++) {}
}
