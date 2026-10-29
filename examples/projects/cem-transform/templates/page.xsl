<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:g="cem:generic-data" version="3.0">
  <xsl:template name="page" match="/">
    <xsl:param name="heading" required="yes"/>
    <main><h1><xsl:value-of select="$heading"/></h1><ul><xsl:for-each select="/g:array/g:object"><li data-order="{normalize-space(g:property[@name='id'])}"><xsl:value-of select="normalize-space(g:property[@name='label'])"/> — <xsl:value-of select="normalize-space(g:property[@name='customer'])"/> (<xsl:value-of select="normalize-space(g:property[@name='total'])"/>)</li></xsl:for-each></ul></main>
  </xsl:template>
</xsl:stylesheet>
