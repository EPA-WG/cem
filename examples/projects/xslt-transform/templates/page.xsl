<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:template name="page">
    <xsl:param name="heading" required="yes"/>
    <xsl:param name="minimum" required="yes"/>
    <main><h1><xsl:value-of select="$heading"/></h1><ul>
      <xsl:for-each select="/orders/order[@status = 'ready' and @total &gt;= $minimum]">
        <li data-order="{@id}"><xsl:value-of select="."/> — <xsl:value-of select="@customer"/> (<xsl:value-of select="@total"/>)</li>
      </xsl:for-each>
    </ul></main>
  </xsl:template>
</xsl:stylesheet>
