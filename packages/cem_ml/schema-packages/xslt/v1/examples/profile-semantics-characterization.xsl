<?xml version="1.0" encoding="UTF-8"?>
<!-- formatter characterization -->
<xsl:stylesheet
  xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
  xmlns:ui="urn:example:ui"
  xmlns:ext="urn:example:ext"
  extension-element-prefixes="ext"
  exclude-result-prefixes="ext"
  version="3.0">
  <xsl:mode name="profile"/>
  <xsl:param name="mode" select="'full'"/>
  <xsl:param name="label" select="'Profile'"/>
  <xsl:template match="/catalog/item[@active = true()]" mode="profile">
    <ui:card class="item-{@id}" data-label="{$label}">
      <xsl:if test="@visible and $mode = 'full'">
        <xsl:text>  fixed text  </xsl:text>
        <![CDATA[foreign <text> & exact]]>
        <xsl:value-of select="normalize-space(title)"/>
        <ext:widget ext:mode="{$mode}">literal extension text</ext:widget>
      </xsl:if>
    </ui:card>
  </xsl:template>
</xsl:stylesheet>
