<xsl:stylesheet
    xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
    version="3.0">
    <xsl:template
        name="report">
        <xsl:param
            name="minimum"
            required="yes"/>
        <report>
      <xsl:for-each select="/orders/order[@status = 'ready' and @total &gt;= $minimum]">
        <item id="{@id}" customer="{@customer}" total="{@total}"><xsl:value-of select="."/></item>
      </xsl:for-each>
    </report>
    </xsl:template>
</xsl:stylesheet>
